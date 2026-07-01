# Crate: wisp-core

#crate #wisp-core

`crates/wisp-core` is the heart of Wisp: a **pure, side-effect-free** library (no file I/O, no
process spawning, no network calls) that owns the data model and knows how to turn it into a
complete [[sing-box Config Model|sing-box config]]. Because it's pure, it's the most thoroughly
unit-tested part of the codebase — **26 tests**, all runnable on any OS with plain
`cargo test -p wisp-core`. See [[Building and Running]] for how to run them, and
[[Architecture Overview]] for how this crate fits with `wisp-engine`/`src-tauri`.

> Callers ([[Crate - wisp-engine]], [[Tauri Backend]], [[Crate - wisp-cli]]) are responsible
> for all I/O: reading input text, writing the generated config to disk, spawning sing-box.

## Modules

| Module | Responsibility |
|---|---|
| `profile.rs` | The `Profile` data model: id, name, list of outbound JSON blobs, active tag. |
| `parse.rs` | `import()`: turn raw sing-box JSON or share links into a `Profile`. |
| `split.rs` | `SplitMode` / `SplitRule` / `SplitConfig`: the split-tunnel data model. |
| `singbox.rs` | `build_config()` / `BuildSettings`: assemble a full sing-box config. |
| `error.rs` | `WispError` (via `thiserror`) and the crate's `Result<T>` alias. |

Public API (re-exported from `lib.rs`):

```rust
pub use error::{Result, WispError};
pub use parse::import;
pub use profile::Profile;
pub use singbox::{build_config, BuildSettings};
pub use split::{SplitConfig, SplitMode, SplitRule};
```

## `Profile` — `profile.rs`

```rust
pub struct Profile {
    pub id: String,
    pub name: String,
    pub outbounds: Vec<serde_json::Value>,
    pub active_tag: Option<String>,
}
```

A profile is a named connection made of **one or more sing-box outbounds**, each stored
*verbatim* as JSON — Wisp never re-serializes protocol details it parsed, it just keeps what it
was given (or produced from a share link). This is deliberate: it avoids Wisp's model getting
out of sync with sing-box's own (frequently-updated) outbound schema.

- `Profile::new(name, outbounds, existing_ids)` derives a deterministic `id` by slugifying
  `name` (`slugify()`, lowercase ASCII, non-alphanumeric runs collapse to `-`) and
  disambiguating against `existing_ids` with a numeric suffix (`foo`, `foo-1`, `foo-2`, ...).
  No randomness involved, so re-importing the same profile text is reproducible.
- `Profile::tags()` returns every outbound's `"tag"` field in order (skipping untagged
  outbounds) — this becomes the sing-box `selector`'s outbound list in
  [[sing-box Config Model|build_config]].

The real-world fixture used across `wisp-core`'s tests (`REAL_CONFIG_FIXTURE` in `parse.rs`) is
a 3-outbound profile: two VLESS+REALITY outbounds (one plain, one with `xtls-rprx-vision` flow)
and one Hysteria2 outbound — see [[Glossary]] for what each of those terms means.

## `import()` — `parse.rs`

```rust
pub fn import(text: &str) -> Result<Profile>
```

Accepts three shapes of input text (auto-detected):

1. A full sing-box `{"outbounds": [...]}` JSON object.
2. A bare `[...]` JSON array of outbounds.
3. One or more `vless://...` / `hysteria2://...` (or `hy2://...`) share links, one per line.

For JSON input, outbounds are filtered to `SUPPORTED_TYPES = ["vless", "hysteria2", "trojan",
"shadowsocks", "vmess"]` — anything else (e.g. `direct`, `block`) is dropped. The profile name
is taken from the first outbound's `tag`, with a trailing sing-box `"§ N"` disambiguation
suffix stripped (`strip_counter_suffix`), e.g. `"Bulgaria, Sophia § 0"` → `"Bulgaria, Sophia"`.

For share links, `parse_vless_link` and `parse_hysteria2_link` hand-build a sing-box outbound
`Value` from the URL's userinfo/host/port/query/fragment, including:

- `vless://`: `uuid`, `flow` (e.g. `xtls-rprx-vision` for [[Glossary#Vision|Vision]]), and TLS
  block including [[Glossary#REALITY|REALITY]] (`pbk`, `sid`) or plain TLS, plus `utls`
  fingerprint and an [[Glossary#XHTTP|XHTTP]] transport block if `type=xhttp`.
- `hysteria2://`/`hy2://`: `password`, optional `obfs` (salamander), and a TLS block
  (`sni`, `insecure`).

This is exactly the logic to extend when adding a new share-link format — see
[[Adding a Protocol or Transport]].

## Split-tunnel model — `split.rs`

```rust
pub enum SplitMode { Off, Exclude, Include }

pub enum SplitRule {
    Process(String),
    ProcessPath(String),
    DomainSuffix(String),
    IpCidr(String),
}

pub struct SplitConfig { pub mode: SplitMode, pub rules: Vec<SplitRule> }
```

`SplitRule::field()` maps a rule to the sing-box route-rule field it becomes, e.g.
`SplitRule::Process("chrome.exe") -> ("process_name", "chrome.exe")`. Full semantics — how
`Off`/`Exclude`/`Include` change the generated `route.rules` and `route.final` — are in
[[Split Tunneling]] (worked example included).

## `build_config()` — `singbox.rs`

```rust
pub struct BuildSettings {
    pub mtu: u32,             // default 1280
    pub clash_secret: String, // default ""
    pub clash_port: u16,      // default 9090
    pub socks_port: Option<u16>,
}

pub fn build_config(
    profile: &Profile,
    split: &SplitConfig,
    settings: &BuildSettings,
) -> Result<serde_json::Value>
```

This is the single most important function in the crate: it assembles a **complete, ready-to-
run sing-box JSON config**. A field-by-field walkthrough of what it produces lives in
[[sing-box Config Model]]; in short, it builds:

- `inbounds`: one `tun` inbound (MTU from settings, `auto_route`/`strict_route: true`,
  `stack: "system"`) plus an optional `socks` inbound if `settings.socks_port` is set.
- `outbounds`: the profile's outbounds **verbatim**, plus a generated `selector` (tag `proxy`)
  listing all of the profile's tags with a sensible `default`, plus `direct` and `block`.
- `route`: rules implementing the active `SplitMode` — see [[Split Tunneling]] — always
  preceded by `{"action": "sniff"}`, a DNS-hijack rule, and a private-IP-to-`direct` rule.
- `dns`: a remote DoT resolver routed through `proxy`, and a local resolver routed `direct`.
- `experimental.clash_api`: `external_controller` bound to `127.0.0.1:<clash_port>` with the
  configured secret, so [[Crate - wisp-engine|wisp-engine]] can query traffic and switch the
  active outbound at runtime via the [[Glossary#Clash API|Clash API]].

Because the whole function is pure (`Profile`/`SplitConfig`/`BuildSettings` in,
`serde_json::Value` out, no I/O), its test suite in `singbox.rs` builds a config from the fixed
fixture profile and asserts on the resulting JSON structure directly — no process spawning or
temp files required. This is what makes 26 tests possible without any Windows/sing-box
dependency.

## Errors — `error.rs`

`WispError` (via `thiserror`) covers only data/format errors, since the crate does no I/O:
`Parse(String)`, `UnsupportedProtocol(String)`, `Json` (from `serde_json::Error`), `Url` (from
`url::ParseError`), `Base64(String)`, and a catch-all `Other(String)`.

## See also

- [[Architecture Overview]] — how this crate fits the bigger picture.
- [[Crate - wisp-engine]] — the layer that actually runs the config this crate produces.
- [[sing-box Config Model]] — deep dive on the generated JSON.
- [[Split Tunneling]] — deep dive on `SplitMode`/`SplitRule` → `route.rules`.
- [[Adding a Protocol or Transport]] — how to extend `parse.rs`/`singbox.rs`.
- [[Glossary]] — REALITY, XHTTP, Vision, Hysteria2, etc.
