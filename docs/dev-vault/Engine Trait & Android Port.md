# Engine Trait & Android Port

#android #engine #architecture

Wisp's README states the goal plainly: *"The engine is hidden behind a Rust `Engine` trait, so
it can later be swapped for an embedded library and shared with an Android build."* This note
explains the seam and what would actually need to change. See [[Crate - wisp-engine]] for the
trait's current (desktop) implementation and [[Architecture Overview]] for the wider picture.

## The trait, today

```rust
#[async_trait]
pub trait Engine: Send + Sync {
    async fn start(&self, config: Value) -> anyhow::Result<()>;
    async fn stop(&self) -> anyhow::Result<()>;
    async fn status(&self) -> EngineStatus;
    async fn stats(&self) -> anyhow::Result<TrafficStats>;
    async fn logs(&self, max_lines: usize) -> Vec<String>;
    async fn switch(&self, tag: &str) -> anyhow::Result<()>;
}
```

Every caller — [[Tauri Backend]]'s commands, [[Crate - wisp-cli]]'s `run` subcommand — depends
only on this trait's methods, never on `SingBoxProcess`'s internals (child process handle, log
ring buffer, Clash HTTP client). The only concrete implementation today is
`wisp_engine::SingBoxProcess` ([[Crate - wisp-engine]]), which:

- Spawns `sing-box(.exe)` as an OS child process.
- Writes the config to a JSON file on disk and points sing-box at it via `-c`.
- Talks to sing-box's Clash API over local HTTP for stats/switch.
- Reads logs by piping the child's stdout/stderr.

None of that is exposed through the trait's signature — `start` takes a `serde_json::Value`
(the config `wisp_core::build_config` produced) and everything else is either a status query or
a control action by tag/line-count. That abstraction is precisely what makes a second
implementation possible without touching `wisp-core`, the command layer's call shape, or the
[[Web UI]]'s `invoke()` contract.

## Why not just always use a child process?

Android doesn't support spawning arbitrary long-lived background binaries the way desktop OSes
do (no writable-and-executable general filesystem for a bundled `.exe`, background process
lifecycle is much more restricted). The standard approach in the sing-box ecosystem for mobile
is to **embed sing-box as a library** compiled from its Go source via `gomobile`, producing a
`.aar` (Android) / `.xcframework` (iOS) that exposes sing-box's core functionality through a
generated JNI/Obj-C bridge, then drive it in-process instead of via a subprocess and HTTP API.

## What a hypothetical `MobileEngine` would look like

A new `wisp-engine` implementation, e.g. `struct MobileEngine { /* JNI handle to libsing-box */ }`,
implementing the same `Engine` trait:

- `start(config)`: instead of writing `config.json` and spawning a process, pass the
  `serde_json::Value` (likely serialized to a string) across the FFI boundary to sing-box's
  embedded `Start`/`Run`-equivalent call.
- `stop()`: call the embedded library's stop/shutdown function directly, no `Child::kill`.
- `stats()`/`switch()`: sing-box's mobile libraries typically expose these as direct function
  calls or callbacks rather than requiring a loopback HTTP call to a Clash API — so
  `MobileEngine` would *not* need `ClashApi` at all, just different FFI calls achieving the same
  outcome.
- `logs()`: read from whatever log sink the embedded library writes to (a callback-fed buffer,
  most likely) instead of a piped stdout/stderr ring buffer.

Because `wisp-core::build_config` already produces a config as data
(`serde_json::Value`, see [[sing-box Config Model]]) rather than anything OS/process-specific,
**it would need no changes at all** to support this — the config shape is the same regardless
of how it's ultimately run.

## What would need to change beyond `wisp-engine`

- **Tauri v2 mobile**: Tauri v2 supports Android/iOS targets from the same codebase, so
  `src-tauri` and the [[Web UI]] would largely carry over — the command layer
  ([[Tauri Backend]]) would need to select `MobileEngine` vs `SingBoxProcess` at compile/runtime
  based on target OS, likely via a `cfg`-gated constructor swap in `state::build_engine`.
- **Resource loading**: [[Crate - wisp-engine]]'s `locate_resources()` (finds `sing-box.exe` +
  `wintun.dll` next to the executable) is Windows/desktop-specific and simply wouldn't apply —
  a mobile build links the sing-box library in at compile time rather than locating a bundled
  binary at runtime.
- **Elevation**: [[Tauri Backend]]'s `elevation.rs` (Windows admin/UAC for TUN adapter creation)
  has no Android equivalent — Android's VPN permission model uses `VpnService` and a user
  consent dialog instead, handled entirely differently (likely needing Android-specific glue
  code outside of what `Engine` abstracts).
- **`wisp-cli`**: stays desktop/CLI-only; it wouldn't run on Android, but doesn't need to.

## See also

- [[Crate - wisp-engine]] — the trait definition and its current desktop implementation.
- [[Architecture Overview]] — why sing-box is wrapped rather than reimplemented in the first
  place (the same reasoning — no mature Rust REALITY/Hysteria2 impl — applies doubly on mobile).
- [[sing-box Config Model]] — the config shape that would carry over unchanged.
- [[Tauri Backend]] — the elevation and resource-location code that's desktop-specific.
