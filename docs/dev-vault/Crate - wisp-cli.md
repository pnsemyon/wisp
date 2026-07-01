# Crate: wisp-cli

#crate #wisp-cli

`crates/wisp-cli` is a small `clap`-based binary (`wisp`) that wires
[[Crate - wisp-core]] and [[Crate - wisp-engine]] together for terminal use. It exists so a
contributor can exercise profile parsing, config generation, and even a real sing-box run
**without the Tauri GUI** — and, for two of its three subcommands, without Windows at all. See
[[Architecture Overview]] for how it relates to the rest of the app.

## Subcommands

### `parse` — inspect a profile

```bash
cargo run -p wisp-cli -- parse path/to/config-or-links.txt
```

Reads the file, calls `wisp_core::import()`, and prints a human-readable summary: profile name,
id, each outbound's tag, and the currently active tag. Useful for sanity-checking a share link
or exported sing-box JSON parses the way you expect — see [[Crate - wisp-core#`import()` —
`parse.rs`|`import()`]].

Example output:
```
profile: Bulgaria, Sophia-7w1t0rtt5a (bulgaria-sophia-7w1t0rtt5a)
outbounds: 3
  - Bulgaria, Sophia-7w1t0rtt5a § 0
  - Bulgaria, Sophia-7w1t0rtt5a § 1
  - Bulgaria, Sophia, hysteria-7w1t0rtt5a § 2
active_tag: Bulgaria, Sophia-7w1t0rtt5a § 0
```

### `gen` — generate a full sing-box config

```bash
cargo run -p wisp-cli -- gen path/to/config.json \
  --mtu 1280 \
  --mode exclude \
  --rule process:chrome.exe \
  --rule domain_suffix:example.com
```

Imports the profile, builds a `SplitConfig` from `--mode` (`off`/`exclude`/`include`) and
repeated `--rule kind:value` flags (`process`, `process_path`, `domain_suffix`, `ip_cidr` — see
[[Split Tunneling]]), calls `wisp_core::build_config()`, and pretty-prints the resulting JSON to
stdout. This is the fastest way to see exactly what config Wisp would hand to sing-box for a
given profile + split settings — pair it with [[sing-box Config Model]] to understand each
field. Runs anywhere, no sing-box binary needed.

### `run` — end-to-end, with a real sing-box process

```bash
cargo run -p wisp-cli -- run path/to/config.json --mtu 1280 --mode off \
  [--binary /path/to/sing-box]
```

Does everything `gen` does, then actually starts it: constructs a
`wisp_engine::SingBoxProcess` (binary auto-detected via `locate_resources()` unless `--binary`
is given), calls `Engine::start`, and prints `TrafficStats` every 2 seconds until Ctrl-C, then
cleanly stops the engine. **This subcommand needs Windows** (TUN adapter creation via
[[Glossary#Wintun|Wintun]]) plus a real `sing-box` binary staged per [[Building and Running]] —
`parse` and `gen` do not.

## Why this crate exists

It's the fastest feedback loop for changes to `wisp-core`/`wisp-engine` logic: no Tauri build,
no webview, no UI to click through. If you're debugging "why doesn't my share link parse right"
or "why does the generated config look wrong", reach for `parse`/`gen` first. If you're
debugging the actual process lifecycle (does sing-box start, does the Clash API respond,
does traffic reporting work), `run` gives you that on a real Windows box without touching the
Tauri backend at all.

## See also

- [[Crate - wisp-core]] — `import`/`build_config`, called directly by `parse`/`gen`.
- [[Crate - wisp-engine]] — `SingBoxProcess`/`locate_resources`, used by `run`.
- [[Split Tunneling]] — the `--mode`/`--rule` flags map directly onto `SplitMode`/`SplitRule`.
- [[Building and Running]] — how to get a real `sing-box` binary for `run`.
- [[Tauri Backend]] — the GUI counterpart that also wires core+engine together.
