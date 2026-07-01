# 05 — Settings and MTU

Open the **Settings** card to see Wisp's two adjustable options: **MTU** and
**Launch at login**. Click **Save settings** after changing anything.

## MTU, in plain terms

**MTU (Maximum Transmission Unit)** is the largest chunk of data your
network connection sends in one piece. VPN tunnels wrap your original data
inside another layer (encryption + tunneling headers), which makes each
packet a bit bigger — if the tunnel's MTU is set too high for the network
path it doesn't fit, and things silently break or slow down.

This is the classic "manual MTU fiddling" problem with other VPN clients:
you'd have to find the tunnel's network adapter and manually lower its MTU
with a command yourself. **Wisp sets this for you automatically** — the
tunnel is created with the right MTU from the start, no shell commands
needed.

### Why 1280 is the safe default

Wisp defaults the tunnel MTU to **1280 bytes**. This is deliberately
conservative — it's small enough to survive almost any network path
(including ones with extra overhead like mobile networks, some Wi-Fi
setups, or double-encapsulated connections) without needing further
adjustment. Trading a little efficiency for "it just works everywhere" is
the right default for a VPN client most people won't want to tune by hand.

### When to change it

You generally don't need to. Consider raising it (e.g. to 1400–1420) only
if:
- You're on a very stable, simple network path and want to squeeze out a
  little extra throughput/efficiency, **and**
- You've confirmed 1280 works fine first and are just optimizing.

Consider lowering it further (e.g. to 1200 or below) if you're on an
unusual network with extra overhead (some mobile carriers, nested VPNs/VMs)
and still see the symptoms below at 1280.

### Symptoms of a wrong MTU

If the MTU is set too high for your network path, you'll typically see:

- Pages that **hang or load partially** — small requests (like the initial
  page HTML) succeed, but anything with a slightly larger payload (images,
  scripts, big API responses) stalls or times out.
- Some sites work fine while others never load, with no obvious pattern.
- Connections that work for a few seconds then stall.

If you see these after connecting, try lowering the MTU (e.g. from 1280 to
1200), save, and reconnect. See also
[06 — Troubleshooting](06-troubleshooting.md).

### How to change it

1. Open **Settings**.
2. Edit the **MTU** field (allowed range: 576–9000).
3. Click **Save settings**.
4. If you're currently connected, disconnect and reconnect for the new MTU
   to take effect (a running tunnel keeps its current MTU until it's
   restarted).

## Autostart

Check **Launch at login** to have Wisp start automatically when you log
into Windows. See [03 — Connecting](03-connecting.md#autostart-launch-at-login)
for details. Remember you'll still see the elevation prompt on every
launch — this is required, not a bug (see
[01 — Installation](01-installation.md#first-launch)).

## Where your config is stored

Wisp saves your profiles, split-tunnel rules, and settings as JSON files in
its per-user app config directory on Windows (the standard per-app
`AppData` location Windows apps use). These persist across restarts and
reconnects automatically — you don't need to re-import your servers every
time you open Wisp.

> **Note:** Everything is stored locally on your machine. See
> [07 — FAQ](07-faq.md) for more on what Wisp does and doesn't send anywhere.

---

Next: [06 — Troubleshooting](06-troubleshooting.md).
