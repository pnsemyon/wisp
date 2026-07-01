# Building and Running

#building #dev-setup

How to build and test each part of Wisp, on any OS or Windows-only where required. See
[[Architecture Overview]] for why the workspace is split the way it is.

## The pure crates — any OS

`wisp-core`, `wisp-engine`, and `wisp-cli` are members of the **root** `Cargo.toml` workspace
and build/test on Linux, macOS, or Windows with nothing extra installed:

```bash
cargo test                    # everything in the root workspace
cargo test -p wisp-core        # just the 26 pure-logic tests — see [[Crate - wisp-core]]
cargo test -p wisp-engine       # process/HTTP-layer tests (dummy binaries, no real sing-box)
```

`wisp-cli`'s `parse` and `gen` subcommands also work with no external binary — see
[[Crate - wisp-cli]]:

```bash
cargo run -p wisp-cli -- parse examples/config.json
cargo run -p wisp-cli -- gen examples/config.json --mode exclude --rule process:chrome.exe
```

Only `wisp-cli run` needs a real `sing-box` binary (see below) and, functionally, Windows (TUN
adapter creation).

## Fetching resources (`sing-box` + `wintun`)

`resources/` (gitignored) needs a pinned `sing-box.exe` and `wintun.dll` before you can actually
*run* the engine (via `wisp-cli run` or the full Tauri app). Two equivalent scripts:

```bash
./scripts/fetch-resources.sh        # bash/curl/unzip — Linux/macOS/WSL
./scripts/fetch-resources.ps1        # PowerShell — native Windows
```

Both:
1. Resolve the latest sing-box release from GitHub (or use a version passed as an argument /
   `-SingboxVersion`).
2. Download `sing-box-<version>-windows-amd64.zip` and extract `sing-box.exe` into
   `resources/`.
3. Download a pinned Wintun build (`0.14.1`) and extract `wintun.dll` into `resources/`.
4. Record the resolved sing-box version in `resources/.singbox-version`.

Note both scripts always fetch the **Windows amd64** sing-box build — Wisp is a Windows app, so
even if you run `fetch-resources.sh` from Linux/WSL to stage resources for a later Windows
build, you get the Windows binary, not a native Linux one. [[Crate - wisp-engine]]'s
`locate_resources()` looks for these files next to the executable, in `./resources`, or in the
repo's `resources/` during dev builds.

## The Windows app

`src-tauri` (see [[Tauri Backend]]) **must be built and run on Windows** — it depends on:
- The [[Glossary#Wintun|Wintun]] driver + [[Glossary#TUN|TUN]] adapter creation, which is
  Windows-specific.
- Administrator elevation (`src-tauri/src/elevation.rs`), needed specifically to create that
  TUN adapter.
- A system webview (Tauri's runtime dependency on Windows' WebView2).

```bash
cargo tauri dev      # dev mode, hot-reloads ui/
cargo tauri build    # produces nsis/msi installers per tauri.conf.json's bundle.targets
```

Both commands relaunch themselves elevated via UAC on startup if not already elevated — see
[[Tauri Backend#Windows elevation — `elevation.rs`|the elevation section]]. Building requires
the `tauri-cli` (`cargo install tauri-cli --version "^2"` or equivalent) and, per Tauri v2's
usual prerequisites, the Microsoft C++ Build Tools and WebView2 runtime.

## Why `src-tauri` is a standalone workspace

Root `Cargo.toml`:

```toml
[workspace]
members = ["crates/wisp-core", "crates/wisp-engine", "crates/wisp-cli"]
exclude = ["src-tauri"]
```

`src-tauri/Cargo.toml` is its **own** `[workspace]` (a "workspace of one"). This split exists so
that `cargo test`/`cargo build` at the repo root — and CI running on Linux — never need a
Windows webview toolchain to succeed. If `src-tauri` were a normal workspace member, any command
touching the whole workspace (`cargo build`, `cargo check`, `cargo test`) would try to compile
it too, and would fail on non-Windows CI runners or dev machines without the Tauri/WebView2
toolchain installed. Keeping it excluded means:

- `wisp-core`/`wisp-engine`/`wisp-cli` stay trivially testable everywhere (this is also why
  `wisp-core` in particular has zero I/O — see [[Crate - wisp-core]] — it was designed to be
  testable without any platform dependency at all).
- `src-tauri` still depends on the workspace crates via **path dependencies**
  (`wisp-core = { path = "../crates/wisp-core" }`), so changes to the core libraries are picked
  up immediately without needing to be published anywhere.

## Release profile

The root workspace sets an aggressive release profile (`opt-level = "s"`, `lto = true`,
`codegen-units = 1`, `strip = true`, `panic = "abort"`) — optimizing for small binary size over
compile time, appropriate for a bundled desktop app where binary size affects installer size and
startup.

## See also

- [[Architecture Overview]] — the crate graph these commands build.
- [[Crate - wisp-core]] / [[Crate - wisp-engine]] / [[Crate - wisp-cli]] — what gets tested/run.
- [[Tauri Backend]] — elevation and persistence details relevant when running the full app.
- [[Contributing]] — conventions to follow when submitting changes (clippy, rustfmt, tests).
- [[Glossary]] — Wintun, TUN.
