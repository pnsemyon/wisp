# 01 — Installation

## Requirements

- **Windows 10 or 11, 64-bit (x64).** Wisp uses a TUN adapter (via Wintun)
  and Windows-specific elevation, so it does not run on macOS or Linux.
- **Administrator rights.** Wisp needs to create a system network adapter to
  route your traffic, which Windows only allows for elevated processes. You
  don't need to *run as* administrator yourself — Wisp will prompt you (see
  below).
- A server to connect to: a `vless://` link, a `hysteria2://`/`hy2://` link,
  or a sing-box JSON config with an `"outbounds"` array. See
  [02 — Adding a server](02-adding-a-server.md).

## Getting the app

Wisp is early in development — there isn't a signed installer download yet.
For now, you build it yourself from source:

1. Install the [Rust toolchain](https://rustup.rs) and [Node.js](https://nodejs.org)
   (needed by Tauri's build tooling).
2. Clone the repository and fetch the bundled runtime pieces Wisp needs
   alongside the app:

   ```powershell
   .\scripts\fetch-resources.ps1
   ```

   This downloads a pinned `sing-box.exe` (the engine that actually speaks
   VLESS/Hysteria2 to your server) and `wintun.dll` (the driver Windows uses
   to create the TUN network adapter) into `resources\`. Wisp can't connect
   to anything without these two files.

3. Build and run the desktop app:

   ```powershell
   cargo tauri dev      # run in development mode
   # or
   cargo tauri build     # produce an installer (nsis/msi) under src-tauri/target
   ```

> **Note:** Steps 2–3 must be run **on Windows** — the desktop app depends on
> Wintun, the TUN adapter, and Windows elevation APIs. The core parsing/config
> logic (`wisp-core`) is plain Rust and can be built and tested on any OS, but
> the full app cannot.

For full build/architecture details, see the
[developer docs](../dev-vault/Home.md) and [`docs/ARCHITECTURE.md`](../ARCHITECTURE.md).

## First launch

1. Start Wisp (double-click the built executable, or the installed
   shortcut once you have an installer).
2. Windows will show a **User Account Control (UAC)** prompt asking to run
   Wisp as administrator. Click **Yes**.

   > **Why does it need this?** Wisp needs to create a TUN network adapter
   > so it can route your system's traffic through the tunnel. Windows only
   > lets administrators create network adapters, so Wisp relaunches itself
   > elevated automatically — you'll see the UAC prompt every time you start
   > it (see [06 — Troubleshooting](06-troubleshooting.md) if this becomes
   > annoying).

3. Once elevated, the Wisp window opens. You'll see:
   - A **Profile** card (empty — "No profiles yet" — until you import a
     server).
   - A big **Connect** button and traffic stats.
   - **Split tunneling** and **Settings** cards.
   - A collapsible **Logs** panel at the bottom.

4. Continue to [02 — Adding a server](02-adding-a-server.md) to import your
   first connection.

> **Note:** Closing the Wisp window does not quit the app — it minimizes to
> the Windows system tray so your tunnel (if connected) keeps running. See
> [03 — Connecting](03-connecting.md#the-system-tray) for how to fully quit.
