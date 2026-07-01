# Contributing

#contributing #conventions

## Repo layout

```
Wisp/
├── crates/wisp-core/     # [[Crate - wisp-core]]
├── crates/wisp-engine/   # [[Crate - wisp-engine]]
├── crates/wisp-cli/      # [[Crate - wisp-cli]]
├── src-tauri/            # [[Tauri Backend]] (standalone workspace — see [[Building and Running]])
├── ui/                   # [[Web UI]]
├── resources/            # gitignored; populated by scripts/fetch-resources.*
├── scripts/               # fetch-resources.sh / .ps1
└── docs/
    ├── ARCHITECTURE.md
    └── dev-vault/         # this vault
```

See [[Architecture Overview]] for the full crate graph and responsibilities.

## Coding conventions

Enforced by CI (`.github/workflows/ci.yml`, `core` job):

```bash
cargo fmt --all -- --check
cargo clippy -p wisp-core -p wisp-engine -p wisp-cli -- -D warnings
cargo test -p wisp-core -p wisp-engine -p wisp-cli
```

- **rustfmt**: standard formatting, no custom `rustfmt.toml` — just run `cargo fmt --all` before
  committing.
- **clippy with `-D warnings`**: all clippy warnings are build failures in CI. Run
  `cargo clippy -p wisp-core -p wisp-engine -p wisp-cli -- -D warnings` locally before pushing.
- **No `unwrap()`/`expect()` in non-test code.** Every current use of `.unwrap()`/`.expect()` in
  the codebase is inside a `#[cfg(test)] mod tests` block. Production code paths return
  `Result`/`anyhow::Result` and propagate errors with `?` or `.context(...)` (see
  `wisp-engine`'s liberal use of `anyhow::Context` for descriptive error chains, and
  `wisp-core`'s `WispError` via `thiserror`). This matters especially in `wisp-core`, which is
  meant to be usable as a library with predictable error behavior, and in `wisp-engine`, where a
  panic would kill the whole Tauri backend process, taking down an active VPN connection with
  it.
- **Tauri commands never panic**: every `#[tauri::command]` in `src-tauri/src/commands.rs`
  returns `Result<T, String>`, converting errors to strings at the boundary so the webview
  always gets a readable message instead of the app crashing — see [[Tauri Backend]].
- **`wisp-core` stays pure**: no file I/O, process spawning, or network calls belong in
  `crates/wisp-core`. If you're adding something that needs I/O, it likely belongs in
  `wisp-engine` or `src-tauri` instead — see [[Crate - wisp-core]] and [[Crate - wisp-engine]].

## Running the full test suite

```bash
cargo test -p wisp-core -p wisp-engine -p wisp-cli   # matches CI exactly
```

All of these run on any OS (no Windows/sing-box binary required) — see
[[Building and Running]] for details, and [[Crate - wisp-core]]/[[Crate - wisp-engine]] for why
each crate's tests don't need real external dependencies (fixture-based pure-function tests in
`wisp-core`; dummy-binary `Engine` tests in `wisp-engine`).

`src-tauri` isn't covered by the `core` CI job (it's a separate `windows-app` job that just
builds, not tests, on `windows-latest`) since it needs a Windows webview toolchain — see
[[Building and Running#Why `src-tauri` is a standalone workspace|the workspace split]].

## Commit / PR expectations

- Keep PRs focused: a change to `wisp-core`'s parsing logic shouldn't also bundle unrelated UI
  tweaks.
- Add or update tests alongside any change to `wisp-core`/`wisp-engine` logic — the pure/testable
  design of this codebase (see [[Architecture Overview]]) only pays off if changes keep the test
  suite meaningful. See [[Adding a Protocol or Transport]] for where new-protocol tests go.
- If you touch the generated sing-box config shape (`wisp-core::singbox.rs`), also check whether
  [[sing-box Config Model]] in this vault needs updating to stay accurate.
- If you touch `#[tauri::command]`s, also check [[Tauri Backend]] and [[Web UI]] (the `invoke()`
  call-map) for accuracy.

## Security: never commit real server credentials

Every test fixture in this repo (`REAL_CONFIG_FIXTURE` in `crates/wisp-core/src/parse.rs`, and
any share links used in tests/examples) uses **example values only**: RFC 5737 documentation IP
addresses (`203.0.113.10`), placeholder UUIDs (`11111111-2222-3333-4444-555555555555`), and
clearly-fake REALITY public keys/short ids/passwords (e.g. `ExamplePublicKeyAAAA...`,
`example-password-1234`). When adding new test fixtures or example configs:

- Never paste a real server's host, port, UUID, REALITY public key/short id, or Hysteria2
  password into a commit, test, issue, or PR — even "for a quick repro." Sanitize first.
- Prefer reusing/extending the existing `REAL_CONFIG_FIXTURE` pattern in `parse.rs` over
  introducing new fixtures with real-looking-but-fake data, so there's one obviously-synthetic
  fixture to audit rather than several.
- The bundled `sing-box.exe`/`wintun.dll` binaries are fetched at build time
  (`scripts/fetch-resources.*`, see [[Building and Running]]) and are gitignored — never commit
  binaries into `resources/`.

## See also

- [[Building and Running]] — how to actually run the commands above.
- [[Architecture Overview]] — the design principles these conventions protect (purity of
  `wisp-core`, the `Engine` seam).
- [[Adding a Protocol or Transport]] — the concrete workflow for a common kind of contribution.
