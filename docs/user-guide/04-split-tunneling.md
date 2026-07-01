# 04 — Split tunneling

Split tunneling lets you choose exactly which traffic goes through the VPN
tunnel and which goes straight out your normal internet connection
("direct"). This is Wisp's headline feature — most clients either tunnel
*everything* or make per-app routing painful to set up.

## The 3 modes

| Mode | What happens | Best for |
|---|---|---|
| **Off** | Everything goes through the VPN. No exceptions. | Simplicity — you want the whole system protected/routed. |
| **Exclude** | The apps/domains you list go **direct** (bypass the VPN); everything else is tunneled. | "Route everything *except* my bank app / my game / one site." |
| **Include** | **Only** the apps/domains you list go through the tunnel; everything else goes direct. | "Only route my browser (or one app) through the VPN — leave the rest of my system alone." |

> **Note:** Private/local network traffic (your home LAN, `192.168.x.x`,
> etc.) always goes direct, in every mode — Wisp never routes local traffic
> through the tunnel, so printers, file shares, and local devices keep
> working normally regardless of your split-tunnel settings.

### Decision table — "I want to…"

| I want to… | Use mode |
|---|---|
| …route absolutely everything through the VPN | **Off** |
| …use the VPN for everything except one or two apps/sites (e.g. banking, a game with anti-cheat issues) | **Exclude** |
| …only send my browser (or one specific app) through the VPN, and leave everything else on my normal connection | **Include** |
| …exclude my local network / LAN devices | Nothing to do — this is automatic in every mode |

## Rule types

A rule is either:

- **An app**, matched by its **process name** (e.g. `chrome.exe`,
  `spotify.exe`) — picked from your currently running processes so you don't
  have to type it by hand.
- **A domain**, matched as a **suffix** (e.g. `netflix.com` matches
  `netflix.com` and any subdomain like `www.netflix.com` or
  `account.netflix.com`).

> **Note:** App rules match by the executable's **name**, not its full file
> path. If two different programs on your machine happen to share an
> executable name (rare, but possible), a rule will affect both. There's no
> way to match by full path from the UI.

## Step-by-step

### Switch modes

1. Open the **Split tunneling** card.
2. Click **Off**, **Exclude**, or **Include**. This takes effect
   immediately (including on an already-running tunnel).

### Add an app

1. Click **+ Add app**.
2. A list of your currently running processes appears. Type in the filter
   box to narrow it down (e.g. type "chrome").
3. Click the process you want (e.g. `chrome.exe`). It's added as a rule
   right away.

> **Tip:** The app must be **running** at the time you open the picker — if
> it's not open yet, start it first, then come back and add it.

### Add a domain

1. Type the domain into the box next to **+ Add domain** (e.g.
   `netflix.com` — no `https://`, no `www.`, no trailing slash).
2. Click **+ Add domain**.

### Remove a rule

Click the **✕** next to any rule in the list to remove it immediately.

## Common recipes

**"Route only my browser through the VPN"**
1. Set mode to **Include**.
2. **+ Add app** → pick your browser's process (e.g. `chrome.exe`,
   `firefox.exe`, `msedge.exe`).
3. Everything else on your PC now goes direct; only that browser's traffic
   uses the tunnel.

**"Everything except Spotify and my games"**
1. Set mode to **Exclude**.
2. **+ Add app** → `spotify.exe`.
3. **+ Add app** → each game's `.exe` (add one at a time while each is
   running).
4. Those apps go direct; everything else — browsing, other apps, background
   traffic — goes through the tunnel.

**"Exclude local/LAN traffic"**
Nothing to configure — Wisp always sends private IP ranges direct,
regardless of mode. If a local device stops being reachable after
connecting, that's a different issue — see
[06 — Troubleshooting](06-troubleshooting.md).

---

Next: [05 — Settings and MTU](05-settings-and-mtu.md).
