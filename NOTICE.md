# Third-party notices

Wisp itself is [MIT-licensed](LICENSE). It bundles one third-party executable that is licensed
separately:

## sing-box-extended (GPLv3)

Wisp bundles a prebuilt binary of
[`shtorm-7/sing-box-extended`](https://github.com/shtorm-7/sing-box-extended) — a fork of
[SagerNet/sing-box](https://github.com/SagerNet/sing-box) that adds Xray-compatible transports
(notably **XHTTP**) on top of the same mainline sing-box config schema — as `sing-box.exe`
alongside the Wisp application.

- `sing-box-extended` is licensed under the **GNU General Public License v3.0 (GPLv3)**.
- Its source code is available at <https://github.com/shtorm-7/sing-box-extended>, and the exact
  binary Wisp bundles corresponds to release
  [`v1.13.14-extended-2.5.0`](https://github.com/shtorm-7/sing-box-extended/releases/tag/v1.13.14-extended-2.5.0).
- Wisp does not statically or dynamically link against `sing-box-extended`; it invokes the
  prebuilt binary as a separate child process over its CLI and local Clash API (HTTP), and the
  two are distributed side by side purely as **mere aggregation** on the same disk/installer.
  Wisp's own MIT-licensed source is not a derivative work of `sing-box-extended` and is not
  itself subject to the GPLv3.
- The binary is not committed to this repository; it's downloaded at build time by
  [`scripts/fetch-resources.sh`](scripts/fetch-resources.sh) /
  [`scripts/fetch-resources.ps1`](scripts/fetch-resources.ps1) directly from the
  `sing-box-extended` GitHub releases, and is bundled into the installer alongside Wisp's own
  binary (see [`docs/RELEASING.md`](docs/RELEASING.md)).

If you redistribute Wisp (or a build of it), you must also make the GPLv3 terms and a pointer to
`sing-box-extended`'s source available, as described above.

## wintun (its own license)

Wisp also bundles `wintun.dll` from the [Wintun](https://www.wintun.net/) project, under its own
license (see <https://www.wintun.net/>). It is fetched by the same scripts referenced above and
distributed alongside Wisp for the same reason: it's a separate driver component, not linked
into Wisp's own code.
