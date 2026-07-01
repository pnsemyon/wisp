<h1 align="center">🌫️ Wisp</h1>

<p align="center">
  <em>A light, stealthy VPN client for Windows — VLESS·REALITY, Vision & Hysteria2 with real per-app split tunneling.</em>
</p>

---

Wisp is a modern Windows client for [sing-box](https://github.com/SagerNet/sing-box)-compatible
servers (VLESS + REALITY / XHTTP, VLESS + Vision, Hysteria2). It wraps
[`shtorm-7/sing-box-extended`](https://github.com/shtorm-7/sing-box-extended) — a fork of
mainline sing-box that adds the Xray transports mainline doesn't have, notably **XHTTP** — in a
clean Rust + Tauri app that fixes the two things that make other clients annoying:

- **🔀 Split tunneling that actually makes sense** — pick exactly which *apps* and *domains*
  go through the tunnel and which stay direct, from a simple UI.
- **📏 No more manual MTU fiddling** — the tunnel MTU is set automatically (default 1280).

> **Why not a pure-Rust engine?** REALITY, XHTTP and Hysteria2 have no mature Rust
> implementations — they live in Go (sing-box/Xray). Reimplementing them would be a
> months-long, security-critical effort. Wisp instead embeds the audited sing-box engine (the
> `sing-box-extended` fork, for XHTTP support) and spends its effort on the UX that's missing.
> The engine is hidden behind a Rust `Engine` trait, so it can later be swapped for an embedded
> library and shared with an Android build.

## Status

🚧 **Early development.** Building toward a first working Windows release.

## Architecture

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md). In short:

| Crate | Role |
|-------|------|
| `wisp-core` | Config model, share-link parsing, sing-box config generation (pure, testable). |
| `wisp-engine` | Runs & controls the sing-box process via its Clash API. |
| `wisp-cli` | Headless test harness for core + engine. |
| `src-tauri` | Tauri v2 desktop app: commands, system tray, state, persistence. |
| `ui/` | Web frontend. |

## Building

`wisp-core` is pure Rust and builds/tests anywhere:

```bash
cargo test -p wisp-core
```

The full Windows app must be built on Windows (needs Wintun + TUN + admin elevation):

```bash
# 1. fetch the pinned sing-box.exe + wintun.dll
./scripts/fetch-resources.sh        # or fetch-resources.ps1 on Windows
# 2. run the app in dev
cargo tauri dev
```

## Roadmap

- [x] Repo scaffold & architecture
- [x] `wisp-core`: parse the 3 protocol links + generate sing-box config
- [x] `wisp-engine`: spawn & control sing-box
- [x] Tauri backend + tray + persistence
- [x] UI: connect, stats, split-tunnel manager
- [ ] Build & smoke-test the app on Windows (Wintun/TUN + elevation)
- [x] Release workflow: bundle engine + build NSIS/MSI installers ([docs](docs/RELEASING.md))
- [ ] **Fix in-place upgrade** so the app binary is replaced even when running in the tray
      ([bug note](docs/dev-vault/Bug%20-%20Stale%20app%20binary%20on%20in-place%20upgrade.md))
- [x] **Rename split modes** Exclude/Include → **Blacklist/Whitelist**
- [x] **Regex** support for app/domain split rules (`domain_regex` / `process_path_regex`)
- [x] Apply split-rule changes to the live tunnel (settings/split edits force a reconnect)
- [x] Gaming split-tunnel recipe: one-click **Exclude Valve / Steam** preset (Dota 2, CS, Steam) that routes Valve's announced networks + Steam domains direct
- [x] Export / import split-tunnel config as JSON
- [x] Configurable log level (error…trace) from the UI
- [ ] Code-signing (installers are currently unsigned)
- [ ] Android target (Tauri v2 mobile + gomobile engine)

## Documentation

- [User guide](docs/user-guide/README.md) — installing Wisp, adding a server, connecting, split tunneling, settings, troubleshooting, and the CLI.
- [Developer docs](docs/dev-vault/Home.md) — architecture, crate layout, and internals.
- [Releasing](docs/RELEASING.md) — how the Windows installers are built and published.

## License

[MIT](LICENSE). Not affiliated with the sing-box project. Wisp bundles
[`shtorm-7/sing-box-extended`](https://github.com/shtorm-7/sing-box-extended) as a separate
executable under its own license (GPLv3) — see [`NOTICE.md`](NOTICE.md) for details.
