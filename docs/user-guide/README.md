# Wisp User Guide

Wisp is a light, stealthy VPN client for **Windows** that connects to
[sing-box](https://github.com/SagerNet/sing-box)-compatible servers using
**VLESS + REALITY** (including XHTTP), **VLESS + Vision**, or **Hysteria2**.
It wraps the battle-tested sing-box engine and adds the two things most
clients get wrong:

- **Split tunneling that actually makes sense** — choose exactly which apps
  and domains go through the tunnel and which stay direct.
- **No manual MTU fiddling** — the tunnel MTU is set automatically (1280 by
  default) so the connection works without you having to tweak network
  settings by hand.

This guide is for anyone using the Wisp desktop app — no networking
background required. If you're looking for build/architecture docs instead,
see the [developer docs](../dev-vault/Home.md).

## Who this is for

You already have a server (a friend, a paid provider, or one you run
yourself) that gives you a `vless://` link, a `hysteria2://` link, or a
sing-box JSON config. Wisp turns that into a one-click "Connect" button on
Windows, with optional per-app/per-domain split tunneling.

## Contents

| Page | What's in it |
|---|---|
| [01 — Installation](01-installation.md) | Requirements, getting the app, first launch, the admin/elevation prompt. |
| [02 — Adding a server](02-adding-a-server.md) | The 3 ways to import a server, multi-server profiles, switching and deleting. |
| [03 — Connecting](03-connecting.md) | Connect/disconnect, the status pill, traffic stats, tray, autostart. |
| [04 — Split tunneling](04-split-tunneling.md) | The headline feature: Off / Exclude / Include modes, adding apps and domains, recipes. |
| [05 — Settings and MTU](05-settings-and-mtu.md) | What MTU means, why 1280, when to change it, where your config lives. |
| [06 — Troubleshooting](06-troubleshooting.md) | Problem → cause → fix table for the most common issues. |
| [07 — FAQ](07-faq.md) | Safety, logging, admin rights, protocols, roadmap. |
| [08 — Power-user CLI](08-power-user-cli.md) | The `wisp` command-line tool for testing configs headlessly. |

> **Note:** Wisp is early-stage software (see the [project status](../../README.md#status)).
> Features described here reflect what's actually implemented today.
