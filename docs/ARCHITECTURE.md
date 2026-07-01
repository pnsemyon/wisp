# Wisp — Architecture

Wisp is a Windows VPN/proxy client (with a future Android target) for VLESS+REALITY,
VLESS+Vision, and Hysteria2 servers. It does **not** re-implement these protocols in Rust —
instead it wraps the [sing-box](https://github.com/SagerNet/sing-box) engine (the same engine
used by Hiddify and the official sing-box apps), and adds the parts that are missing or
inconvenient in existing clients:

1. **Per-app / per-domain split tunneling** with a friendly UI.
2. **Automatic MTU handling** (no manual `ip link set mtu` steps).
3. A clean Tauri v2 (Rust + web) client that also compiles for Android later.

## Crate / module layout

```
Wisp/
├── crates/
│   ├── wisp-core/     # Pure logic, no I/O side effects. The heart of the app.
│   ├── wisp-engine/   # Runs & controls the sing-box engine process.
│   └── wisp-cli/      # Headless CLI for testing core+engine without a GUI.
├── src-tauri/         # Tauri v2 desktop backend (commands, tray, state).
├── ui/                # Web frontend (vanilla HTML/CSS/JS to start).
├── resources/         # Downloaded sing-box.exe + wintun.dll (gitignored).
└── scripts/           # fetch-resources, build helpers.
```

### wisp-core (no side effects, unit-testable)

Responsible for the data model and for **generating a complete sing-box config**.

- `profile.rs` — `Profile`, `Outbound` model. A profile is a named connection with one or
  more outbounds (the user's link already contains 3). Serde (de)serializable.
- `parse.rs` — Import profiles from:
  - Raw sing-box `{"outbounds":[...]}` JSON (the user already has this).
  - `vless://` and `hysteria2://` share links.
  - (later) subscription URLs.
- `split.rs` — Split-tunnel model:
  - `SplitMode`: `Off` (everything through proxy), `Exclude` (listed apps/domains go
    DIRECT, rest proxied), `Include` (only listed apps/domains proxied, rest DIRECT).
  - `SplitRule`: `Process(String)` (e.g. `chrome.exe`), `ProcessPath(String)`,
    `DomainSuffix(String)`, `IpCidr(String)`.
- `singbox.rs` — `build_config(profile, split, settings) -> serde_json::Value` producing a
  full sing-box config:
  - `inbounds`: one `tun` inbound with `mtu` (default **1280**, configurable),
    `auto_route: true`, `strict_route: true`, `stack: "system"`, and
    `endpoint_independent_nat: false`.
  - `outbounds`: the profile's proxy outbound(s) + a `direct` + a `block` + a `dns` outbound,
    plus a `selector` so the UI can switch active server.
  - `route`: rules implementing the split mode (map SplitRules to `process_name` /
    `process_path` / `domain_suffix` / `ip_cidr` → `direct` or the selector).
  - `experimental.clash_api`: `{ external_controller: "127.0.0.1:9090", secret: <random> }`
    so wisp-engine can query traffic + switch outbounds at runtime.
  - `dns`, `log` blocks with sane defaults.
- `error.rs` — `WispError` via `thiserror`.

### wisp-engine (side effects: spawns process, HTTP)

- `engine.rs` — `trait Engine { async fn start(&self, config: Value) -> Result<()>;
  async fn stop(&self) -> Result<()>; async fn status(&self) -> EngineStatus;
  async fn stats(&self) -> TrafficStats; }` — abstraction so we can swap the bundled
  `.exe` for an embedded `libsing-box` (FFI/gomobile) on Android later.
- `singbox_process.rs` — `SingBoxProcess` impl: writes the config to a temp file, spawns
  `sing-box.exe run -c <file>` (elevated on Windows), captures logs, kills cleanly on stop.
- `clash_api.rs` — thin client for sing-box's Clash API: `GET /traffic` (up/down),
  `GET /connections`, `PUT /proxies/<selector>` to switch active server.
- `resources.rs` — locate `sing-box.exe` + `wintun.dll` next to the app.

### src-tauri (Tauri v2 backend)

Tauri commands exposed to the UI:
`import_profile(text)`, `list_profiles()`, `delete_profile(id)`, `connect(profile_id)`,
`disconnect()`, `status()`, `traffic()`, `get_split()`, `set_split_mode(mode)`,
`add_split_rule(rule)`, `remove_split_rule(rule)`, `list_running_processes()` (to help the
user pick apps), `settings_get/set`. Holds `AppState { engine, profiles, split, settings }`
behind a `tokio::Mutex`. Provides a **system tray** (connect/disconnect/quit) and persists
profiles+settings to the OS config dir. Requests **admin elevation** (needed to create the
TUN adapter) via a manifest.

### ui (web frontend)

Single-page: server/profile selector, big Connect toggle, live up/down speed + totals,
split-tunnel panel (mode radio + add/remove apps and domains), log viewer, settings (MTU,
autostart). Vanilla JS + fetch of Tauri `invoke` to start; can move to a framework later.

## Split tunneling — how it maps to sing-box

sing-box captures all traffic via the TUN inbound (`auto_route`). `route.rules` are evaluated
top-to-bottom; first match wins; each rule sets an `outbound`.

- **Exclude mode**: emit rules `{ process_name/domain_suffix/... : <listed>, outbound: "direct" }`
  first, then a final default → the `proxy` selector.
- **Include mode**: emit rules for listed items → `proxy` selector, final default → `direct`.
- Always keep `{ ip_is_private: true, outbound: "direct" }` and a DNS hijack rule at the top.

## MTU handling

The `mtu` is written directly into the TUN inbound (default 1280, which is what the user was
setting by hand). No post-launch shell commands. Exposed as a setting.

## Build / run

- Dev on any OS for `wisp-core` (pure Rust, `cargo test`).
- The Windows app (`src-tauri`) must be built/run on Windows (Wintun + TUN + elevation).
- `scripts/fetch-resources.*` downloads pinned `sing-box.exe` + `wintun.dll` into `resources/`.
