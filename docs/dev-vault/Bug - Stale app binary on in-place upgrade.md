# Bug — stale `wisp-app.exe` on in-place upgrade (v0.1.0 → v0.1.1)

Status: **diagnosed, not yet fixed** (2026-07-02). Related: [[Building and Running]],
[[sing-box Config Model]], [[Crate - wisp-core]], [[Tauri Backend]], [[Home]].

## Symptom
After "upgrading" from v0.1.0 to v0.1.1, Connect still failed exactly like before:
sing-box starts and immediately exits (`exit code 1`), no connection.

## Root cause
The v0.1.1 installer upgraded the **bundled engine but not the app binary**, producing a
mismatched ("Frankenstein") install:

- `resources/sing-box.exe` → **updated** to the xhttp fork (`sing-box-extended`, 74,158,080 B).
- `wisp-app.exe` → **NOT updated**, still **v0.1.0** (embedded `FileVersion/ProductVersion = 0.1.0`).

Because `wisp-app.exe` (which contains `wisp-core`'s `build_config`) was still v0.1.0, it kept
generating the **old sing-box config schema** that the new engine rejects:

- legacy DNS servers (`{"address":"tls://8.8.8.8"}`) — **fatal** in sing-box ≥1.12
- a `block` outbound — removed in 1.12
- xhttp transport with Xray **camelCase** fields (`xPaddingBytes`) — ignored by the engine, so
  `x_padding_bytes` stays at its zero default (also fatal for xhttp)

sing-box 1.13.14 (the fork's base) refuses to start on the legacy DNS block:
`FATAL ... legacy DNS servers is deprecated ... set ENABLE_DEPRECATED_LEGACY_DNS_SERVERS=true`.

## Why the app binary wasn't replaced
**Wisp was still running in the system tray during the install.** Closing the main window only
*hides it to the tray* (`WindowEvent::CloseRequested` → `prevent_close()` + `hide()` in
`src-tauri/src/lib.rs`), so the v0.1.0 process was alive and holding a lock on `wisp-app.exe`.
Windows cannot overwrite a running executable, so NSIS skipped it while still replacing the
non-running `resources/*` files.

## Evidence (from the user's machine)
- `wisp-app.exe` embedded version strings → `0.1.0`.
- `resources/sing-box.exe` size = 74,158,080 B = byte-identical to the fork's Windows build.
- Freshly generated `%APPDATA%/com.wisp.app/config.json` → legacy DNS, `block` outbound present,
  xhttp transport still camelCase → proves old `build_config`.
- `%LOCALAPPDATA%/Wisp/logs/wisp.log.*`:
  `WARN singbox: FATAL ... legacy DNS servers ...` then
  `health check failed err=sing-box exited early with status exit code: 1`.

## The v0.1.1 fix itself is correct
`wisp-cli gen` built from v0.1.1 code produces the new schema and passes `sing-box check`
against the real fork binary for Off/Exclude/Include, with xhttp kept + normalized
(`x_padding_bytes: "100-1000"`). The logic is right; it just never ran on the user's machine.

## Immediate workaround (for testing)
Fully **quit Wisp from the tray** (or kill `wisp-app.exe`), **uninstall**, confirm
`%LOCALAPPDATA%/Wisp` and `%APPDATA%/com.wisp.app` are gone, then install a fresh build. A clean
v0.1.1 install pairs the correct app with the correct engine.

## Fix options (to implement later — do NOT auto-fix yet)
1. **Make the installer close the running app first.** Tauri NSIS supports hooks / a
   "kill running app" step; ensure the tray process is terminated before file replacement, or
   prompt the user. Investigate `tauri.conf.json > bundle.windows.nsis` install-mode + hooks.
2. **Guard against schema/engine mismatch at runtime.** Have the app record/verify a
   config-schema version compatible with the bundled engine, or run `sing-box check` on the
   generated config before `run` and surface a clear error instead of a silent exit.
3. **Reconsider hide-to-tray as the close action**, or make "Quit" prominent, so users don't
   leave a stale process locking the binary during upgrades.
4. **Single-instance guard** so a second launch doesn't race the first.

## Process lesson (meta)
v0.1.1 was released via CI and declared "working" without testing an actual clean install of
the produced installer, and earlier validation used a *sanitized* copy of the user's config
rather than the real one. Going forward: build, install cleanly, and verify the real config end
to end **before** telling the user it works. See [[Building and Running]] for the intended
local build + test loop.
