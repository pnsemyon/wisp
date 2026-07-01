# 07 — FAQ

**Is Wisp safe? Does it log my traffic?**

Wisp itself doesn't collect or transmit any telemetry, analytics, or usage
data — there's no server-side component of Wisp that your traffic or
activity is reported to. Your servers, split-tunnel rules, and settings are
stored only on your own machine (see
[05 — Settings and MTU](05-settings-and-mtu.md#where-your-config-is-stored)).
Wisp does keep a local, in-memory log of the sing-box engine's own output
(visible in the **Logs** panel) to help you diagnose connection problems —
this is not sent anywhere; it's just shown in the app for your own
troubleshooting. Your actual privacy while connected (whether *your VPN
provider* logs your traffic) depends on the server/provider you connect to,
not on Wisp.

**Why does Wisp need administrator rights?**

Routing all of your system's traffic through a VPN tunnel requires creating
a virtual network adapter (a TUN device via Wintun) and changing Windows'
routing table — both are operations Windows restricts to administrators.
Wisp requests elevation automatically on launch so you don't have to
right-click → "Run as administrator" yourself. See
[01 — Installation](01-installation.md#first-launch).

**How is Wisp different from Hiddify (or other sing-box-based clients)?**

Wisp is built around the same underlying engine (sing-box) that Hiddify and
similar clients use, but focuses specifically on:
- A split-tunnel UI built around picking apps (from your actual running
  processes) and domains directly, with clear Off/Exclude/Include modes.
- Automatic MTU handling, so you never have to manually adjust adapter MTU
  yourself.
- A minimal, Windows-first client scoped to VLESS (REALITY/Vision) and
  Hysteria2 rather than every protocol sing-box supports.

**Does split tunneling work by app, or only by domain?**

Both. You can add rules by **app** (matched by running process name, e.g.
`chrome.exe`, picked from a live list of running processes) and by
**domain** (matched as a suffix, e.g. `netflix.com`). See
[04 — Split tunneling](04-split-tunneling.md#rule-types).

**What protocols are supported?**

- **VLESS + REALITY** (including the **XHTTP** transport)
- **VLESS + Vision** (REALITY with the `xtls-rprx-vision` flow)
- **Hysteria2**

These are the protocols Wisp's import flow, UI, and testing are built
around. See [02 — Adding a server](02-adding-a-server.md).

**Will there be an Android version?**

It's on the roadmap (see the root [README](../../README.md#roadmap)) — the
engine is deliberately kept behind an abstraction so it can later be swapped
for an embedded/mobile build and shared with an Android app. There's no
Android release today.

**Is my config or credentials (UUIDs, passwords, server addresses) sent
anywhere?**

No. Everything you paste when adding a server — UUIDs, passwords, server
hostnames, REALITY keys, etc. — is parsed and stored locally on your
machine, and used only to build the local configuration file that the local
sing-box process reads. Wisp has no backend server of its own; the only
network traffic that leaves your machine because of Wisp is the traffic
sing-box sends to *your chosen server* once you connect.

---

Power users: see [08 — Power-user CLI](08-power-user-cli.md) for testing
configs without the GUI.
