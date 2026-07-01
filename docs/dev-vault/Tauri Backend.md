# Tauri Backend

#tauri #src-tauri

`src-tauri` is the Tauri v2 desktop backend: it wires up persisted state, the sing-box engine
from [[Crate - wisp-engine]], every `#[tauri::command]` the [[Web UI]] calls, a system tray, and
Windows admin elevation. It's a **standalone Cargo workspace** (see [[Building and Running]] for
why) that depends on both [[Crate - wisp-core]] and [[Crate - wisp-engine]].

## `AppState` — `state.rs`

```rust
pub struct AppState {
    pub inner: Mutex<Inner>,   // tokio::sync::Mutex
    pub config_dir: PathBuf,
}

pub struct Inner {
    pub profiles: Vec<Profile>,
    pub active_profile: Option<String>,
    pub split: SplitConfig,
    pub settings: Settings,
    pub engine: Arc<SingBoxProcess>,
}
```

Everything mutable lives behind one `tokio::Mutex<Inner>`, managed by Tauri (`app.manage(state)`
in `lib.rs`) and accessed in commands via `tauri::State<'_, AppState>`. `AppState::new(config_dir)`
loads each persisted file (falling back to `Default` if missing/unparseable) and constructs the
initial engine handle via `build_engine()`.

`Settings` (persisted, `settings.json`):

```rust
pub struct Settings {
    pub mtu: u32,             // default 1280
    pub autostart: bool,      // default false
    pub clash_port: u16,      // default 9090
    pub clash_secret: String, // deterministic per-install, not random
}
```

The Clash secret is deterministically derived from the app identifier rather than randomly
generated — good enough to keep the local API from being trivially guessed by another local
process, while staying stable across restarts without needing a CSPRNG dependency or extra
persisted secret.

## Persistence

Three JSON files live in the OS app-config directory (`app.path().app_config_dir()` —
typically `%APPDATA%/com.wisp.app` on Windows):

| File | Contents | Written by |
|---|---|---|
| `profiles.json` | `ProfilesFile { profiles: Vec<Profile>, active_profile: Option<String> }` | `save_profiles()` |
| `settings.json` | `Settings` | `save_settings()` |
| `split.json` | `SplitConfig` | `save_split()` |

Kept as three separate files (rather than one blob) so each concern can be loaded/saved
independently; `active_profile` is bundled into `profiles.json` (not its own file) specifically
so that re-selecting a profile survives an app restart alongside the profile list itself.

## Commands — `commands.rs`

All 17 commands return `Result<T, String>` (never panic) so the webview always gets a readable
error message. Grouped by concern:

**Profiles**
- `import_profile(text) -> Profile` — calls `wisp_core::import`, dedupes the id against existing
  profiles (`dedupe_id`, mirrors `wisp_core::profile`'s private `unique_id` logic), appends and
  persists.
- `list_profiles() -> Vec<Profile>`
- `delete_profile(id)` — also clears `active_profile` if it pointed at the deleted profile.
- `set_active_profile(id)` — errors if `id` doesn't exist.

**Connection**
- `connect() -> EngineStatus` — the core flow: look up the active profile, build
  `BuildSettings` from current `Settings`, call `wisp_core::build_config`, stop any previously
  running engine, **rebuild** the `SingBoxProcess` (via `build_engine`, so it always reflects
  the latest Clash port/secret), `start()` it, and return its status.
- `disconnect()` — `engine.stop()`.
- `status() -> EngineStatus`
- `traffic() -> TrafficStats`
- `logs(n) -> Vec<String>`
- `switch_outbound(tag)` — calls `Engine::switch`, then updates the active profile's
  `active_tag` and persists it.

**Split tunneling**
- `get_split() -> SplitConfig`
- `set_split_mode(mode)`
- `add_split_rule(rule)` — no-ops if the rule is already present (dedup by equality).
- `remove_split_rule(rule)`

**Settings**
- `get_settings() -> Settings`
- `set_settings(settings)` — only rebuilds the engine handle immediately if nothing is
  currently running; if connected, the new settings (MTU, Clash port/secret) take effect on the
  *next* `connect()` rather than pulling the rug out from under a live process (rebuilding while
  connected would point `status`/`stats`/`switch` at a port the running sing-box process isn't
  actually listening on).

**Misc**
- `list_running_processes() -> Vec<String>` — cross-platform via `sysinfo`, used to populate
  the "Add app" picker in the [[Web UI]]'s split-tunnel panel; run on a blocking thread
  (`spawn_blocking`) since `sysinfo` is synchronous.

See [[Split Tunneling]] and [[sing-box Config Model]] for what `connect()`'s generated config
actually contains, and [[Crate - wisp-engine]] for what `Engine::start/stop/status/stats/logs/
switch` do underneath.

## System tray — `lib.rs`

A tray icon with a 4-item menu: **Connect**, **Disconnect**, **Show Wisp**, **Quit**.
Connect/Disconnect spawn an async task that calls the same `commands::connect`/`disconnect`
functions the UI uses. Left-clicking the tray icon (or "Show Wisp") re-shows and focuses the
main window. Closing the main window (`WindowEvent::CloseRequested`) is intercepted
(`api.prevent_close()`) to just **hide** the window instead of exiting — the app (and any active
tunnel) keeps running until "Quit" is chosen explicitly.

## Windows elevation — `elevation.rs`

Creating the [[Glossary#Wintun|Wintun]]/[[Glossary#TUN|TUN]] adapter that sing-box's `tun`
inbound relies on **requires Administrator rights** on Windows. `ensure_elevated()` is called
first thing in `run()` (`lib.rs`):

- On Windows: checks the current process token via `GetTokenInformation`/`TokenElevation`
  (`is_elevated()`). If not elevated, calls `ShellExecuteW` with the `"runas"` verb to relaunch
  the same executable with a UAC prompt (`relaunch_elevated()`), then returns `false` so the
  original, non-elevated process can exit quietly without ever building the Tauri app.
- On non-Windows: `ensure_elevated()` is a no-op that always returns `true`, so the module is
  `cfg(windows)`-gated end to end and callers don't need their own `cfg` branches.

This is why `run()`'s very first line is `if !elevation::ensure_elevated() { return; }` — every
other line of backend setup only executes in the elevated instance.

## See also

- [[Crate - wisp-core]] / [[Crate - wisp-engine]] — the libraries this backend is built on.
- [[Web UI]] — the frontend that calls every command listed above.
- [[Split Tunneling]] — the model behind `get_split`/`set_split_mode`/`add_split_rule`.
- [[Building and Running]] — how `src-tauri` builds differently from the pure crates.
- [[Glossary]] — Wintun, TUN, Clash API, MTU.
