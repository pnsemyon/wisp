# Icons

`icon.png`, `128x128.png`, `128x128@2x.png`, `32x32.png`, and `icon.ico` in this
directory are real, valid image files (a simple rounded-square "W" mark),
generated on Linux with Pillow so `cargo check`/`cargo tauri dev` has
something real to load for the window/tray icon.

They are good enough to build and run, but are **not** final artwork. Before
shipping a real Windows build, regenerate the full icon set from better
source art:

```powershell
cargo tauri icon path\to\source-icon.png
```

Notes:
- `icon.icns` (macOS) is intentionally **not** included: Wisp only targets
  Windows (see `tauri.conf.json` `bundle.targets`), and `cargo tauri build`
  does not require it for a Windows-only bundle. Add it later if a macOS
  target is introduced (`cargo tauri icon` will generate it from the same
  source PNG).
- The build must not hard-fail for lack of a *complete* icon set at
  code-review time — `icon.png`/`icon.ico` are present and valid, which is
  what Tauri needs to compile and to build a Windows bundle.
