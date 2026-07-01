<h1 align="center">🌫️ Wisp</h1>

<p align="center">
  <em>A light, stealthy VPN client for Windows — VLESS·REALITY, Vision & Hysteria2 with real per-app split tunneling.</em>
</p>

---

Wisp is a modern Windows client for [sing-box](https://github.com/SagerNet/sing-box)-compatible
servers (VLESS + REALITY / XHTTP, VLESS + Vision, Hysteria2). It wraps the battle-tested
sing-box engine and wraps it in a clean Rust + Tauri app that fixes the two things that make
other clients annoying:

- **🔀 Split tunneling that actually makes sense** — pick exactly which *apps* and *domains*
  go through the tunnel and which stay direct, from a simple UI.
- **📏 No more manual MTU fiddling** — the tunnel MTU is set automatically (default 1280).

> **Why not a pure-Rust engine?** REALITY, XHTTP and Hysteria2 have no mature Rust
> implementations — they live in Go (sing-box/Xray). Reimplementing them would be a
> months-long, security-critical effort. Wisp instead embeds the audited sing-box engine and
> spends its effort on the UX that's missing. The engine is hidden behind a Rust `Engine`
> trait, so it can later be swapped for an embedded library and shared with an Android build.

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
- [ ] Code-signing (installers are currently unsigned)
- [ ] Android target (Tauri v2 mobile + gomobile engine)

## Documentation

- [User guide](docs/user-guide/README.md) — installing Wisp, adding a server, connecting, split tunneling, settings, troubleshooting, and the CLI.
- [Developer docs](docs/dev-vault/Home.md) — architecture, crate layout, and internals.
- [Releasing](docs/RELEASING.md) — how the Windows installers are built and published.

## License

[MIT](LICENSE). Not affiliated with the sing-box project; sing-box is bundled under its own license.
