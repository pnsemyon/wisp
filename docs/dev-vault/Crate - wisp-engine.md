# Crate: wisp-engine

#crate #wisp-engine

`crates/wisp-engine` is the layer that actually *runs* things: it spawns the sing-box process,
writes its config to disk, reads its logs, and talks to its HTTP
[[Glossary#Clash API|Clash API]]. Everything in [[Crate - wisp-core]] is pure; everything here
has side effects. See [[Architecture Overview]] for how it fits in.

## Modules

| Module | Responsibility |
|---|---|
| `engine.rs` | The `Engine` trait + `EngineState`/`EngineStatus`/`TrafficStats` types. |
| `singbox_process.rs` | `SingBoxProcess`: the only current `Engine` impl, backed by a child process. |
| `clash_api.rs` | `ClashApi`: thin HTTP client for sing-box's Clash-compatible API. |
| `resources.rs` | `locate_resources()`: find the bundled `sing-box(.exe)` + `wintun.dll`. |

## The `Engine` trait — `engine.rs`

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

This is the seam described in [[Engine Trait & Android Port]]: callers ([[Tauri Backend]],
[[Crate - wisp-cli]]) only ever depend on this trait, never on `SingBoxProcess` directly (except
to construct one). Supporting types:

- `EngineState`: `Stopped | Starting | Running | Errored`.
- `EngineStatus { state, active_tag, since_unix, last_error }` — a full status snapshot.
- `TrafficStats { up_bytes, down_bytes, up_speed, down_speed }` — cumulative totals plus
  instantaneous speed (bytes/sec), all `u64`.

## `SingBoxProcess` — `singbox_process.rs`

The lifecycle, in order:

1. **`start(config)`**: sets state to `Starting`, then calls `spawn_and_confirm`:
   - Creates `work_dir` if missing, writes `config` (pretty-printed) to `work_dir/config.json`.
   - Spawns `sing-box run -c config.json -D <work_dir>` (elevated on Windows, since creating
     the [[Glossary#Wintun|Wintun]]/[[Glossary#TUN|TUN]] adapter needs admin — see
     [[Tauri Backend]] for the elevation flow) with piped stdout/stderr and `kill_on_drop(true)`.
   - Spawns a background task per stream (`spawn_log_reader`) that reads lines and pushes them
     into a shared **ring buffer** (`VecDeque`, capacity `LOG_RING_CAPACITY = 500`), dropping
     the oldest line once full.
   - Unless disabled (`health_check: false`, test-only), polls `GET /version` on the Clash API
     up to 10 times with exponential backoff (200ms → capped at 2s) to confirm sing-box came up
     healthy, bailing out early if the child process already exited.
   - On success: records `since_unix`, stores the `Child` handle, sets state `Running`.
   - On failure: sets state `Errored` with `last_error` set to the error string.
2. **`stop()`**: if a child is stored, `start_kill()` + `wait()` on it (clean kill, not a
   signal race), then resets state to `Stopped`.
3. **`status()`**: returns a clone of the current `EngineStatus`.
4. **`stats()`**: calls `ClashApi::connections()` for the current `downloadTotal`/`uploadTotal`,
   then computes `up_speed`/`down_speed` as `(current_total - previous_total) / elapsed_secs`
   against the last sampled totals (`last_sample: Option<(u64, u64, Instant)>`), i.e. **speed is
   derived from two totals samples over time**, not read directly from sing-box.
5. **`logs(max_lines)`**: returns the last `max_lines` entries from the ring buffer.
6. **`switch(tag)`**: calls `ClashApi::switch_selector("proxy", tag)` to change which outbound
   the generated `selector` (see [[sing-box Config Model]]) currently points at, then records
   `active_tag` locally.

`SingBoxProcess::new(binary, work_dir, clash_port, clash_secret)` is the real constructor;
`new_for_test` (test-only) lets tests point at an arbitrary binary/args (e.g. `/bin/sh -c "sleep
30"`) and disables the health check, since a dummy binary has no HTTP API to poll. This is how
`singbox_process.rs`'s tests exercise the full start/stop lifecycle without an actual sing-box
binary or Windows.

## `ClashApi` — `clash_api.rs`

A minimal `reqwest`-based client (3s timeout) bound to `http://127.0.0.1:<port>`, with an
optional bearer-auth header (only attached if `secret` is non-empty — sing-box itself only
requires the header when a secret is configured):

| Method | Endpoint | Purpose |
|---|---|---|
| `version()` | `GET /version` | Cheap health check used by `wait_for_health`. |
| `connections()` | `GET /connections` | Returns `ConnectionsSnapshot { download_total, upload_total, connections: Vec<Value> }` (camelCase JSON fields renamed via serde). |
| `switch_selector(selector, name)` | `PUT /proxies/<selector>` `{"name": <name>}` | Change the active outbound of a `selector`-type outbound (used for the `proxy` selector). |

## `locate_resources()` — `resources.rs`

Finds the platform binary name (`sing-box.exe` on Windows, `sing-box` elsewhere) and
`wintun.dll`, searching in order:

1. The directory containing the current executable.
2. `./resources` (relative to CWD).
3. `<CARGO_MANIFEST_DIR>/../../resources` (repo-root `resources/`, for dev builds).

It's fine for `sing-box` and `wintun.dll` to live in *different* directories among these — the
first hit for each wins independently (`search_dirs`, tested with injected temp dirs). See
[[Building and Running]] for how `resources/` gets populated
(`scripts/fetch-resources.sh`/`.ps1`).

## Testing without Windows

Every test in this crate either uses `new_for_test` (dummy binary, no health check) or plain
unit assertions (`ClashApi`'s header logic, `resources.rs`'s `search_dirs` against temp dirs) —
none require a real `sing-box.exe`, so `cargo test -p wisp-engine` runs on Linux/macOS/CI too.
Only the real end-to-end path (spawning the actual bundled binary and creating a TUN adapter)
needs Windows; see [[Crate - wisp-cli]]'s `run` subcommand and [[Building and Running]].

## See also

- [[Engine Trait & Android Port]] — why the trait exists and what an Android port needs.
- [[Crate - wisp-core]] — produces the `Value` config this crate consumes.
- [[Tauri Backend]] — the main consumer of `Engine`/`SingBoxProcess` on desktop.
- [[Crate - wisp-cli]] — the other consumer, for headless testing.
- [[sing-box Config Model]] — what's actually in the config being run.
- [[Glossary]] — Clash API, Wintun, TUN, MTU.
