# Releasing Wisp

Wisp ships as Windows installers (an NSIS `-setup.exe` and an `.msi`) built by
the [`release.yml`](../.github/workflows/release.yml) GitHub Actions workflow.

## How a release is built

1. The workflow runs on `windows-latest`.
2. [`scripts/fetch-resources.ps1`](../scripts/fetch-resources.ps1) downloads the
   pinned `sing-box.exe` and `wintun.dll` into `resources/`.
3. [`tauri-action`](https://github.com/tauri-apps/tauri-action) runs
   `cargo tauri build` in `src-tauri/`, which:
   - compiles the Rust app,
   - **bundles `sing-box.exe` + `wintun.dll`** into the installer (see
     `bundle.resources` in [`src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json) —
     they install to `resources/` next to the app), and
   - produces the NSIS and MSI installers.
4. The installers are uploaded to a **draft GitHub Release** for the tag.

At runtime the app tells `wisp-engine` where the binaries live by setting the
`WISP_RESOURCE_DIR` env var to Tauri's resource directory; `locate_resources()`
looks there (and in `resources/` beside the exe) first. So the installed app
finds the engine without any PATH setup.

## Cutting a release

**Option A — push a tag** (preferred):

```bash
# bump the version in src-tauri/tauri.conf.json and the crate Cargo.tomls first
git tag v0.1.0
git push origin v0.1.0
```

**Option B — manual**: Actions tab → **Release** workflow → *Run workflow* →
enter the tag (e.g. `v0.1.0`).

Then go to the repo's **Releases**, review the draft, and publish it.

## Versioning

Keep these in sync when bumping a version:

- `src-tauri/tauri.conf.json` → `version`
- `Cargo.toml` (workspace) → `workspace.package.version`
- `src-tauri/Cargo.toml` → `version`
- the git tag (`vX.Y.Z`)

## Not done yet

- **Code signing.** Installers are unsigned, so SmartScreen warns on first run.
  To sign, add a code-signing certificate and set Tauri's
  `bundle.windows.certificateThumbprint` / signing env vars in the workflow.
- **Auto-update.** `includeUpdaterJson` is off; wire up the Tauri updater plugin
  if/when we want in-app updates.
