# 03 — Connecting

## Connect / disconnect

1. Make sure a profile (and, if it has several, a server) is selected — see
   [02 — Adding a server](02-adding-a-server.md).
2. Click the big **Connect** button.
3. Windows routes all system traffic through the tunnel once the status
   pill reads **Running**.
4. Click the same button (now labeled **Disconnect**) to stop the tunnel.

> **Note:** You need an active profile before connecting — if none is
> selected, Wisp shows "Import and select a profile first."

## Reading the status pill

The pill in the top-right of the window always reflects the real state of
the underlying sing-box engine:

| Status | Meaning |
|---|---|
| **Stopped** | Not connected. All traffic goes direct (normal internet, no tunnel). |
| **Starting** | Connect was pressed; sing-box is launching and the TUN adapter is being set up. The button shows "Connecting…". |
| **Running** | Connected — the tunnel is up and traffic is flowing through it. |
| **Errored** | Sing-box failed to start or crashed. Open the **Logs** panel (below) to see why, and check [06 — Troubleshooting](06-troubleshooting.md). |

## Traffic stats

While **Running**, the Connect card shows:

- **↓ Down** — current download speed and total bytes downloaded this
  session.
- **↑ Up** — current upload speed and total bytes uploaded this session.

These update roughly every 1.5 seconds and reset to `0 B/s` once you
disconnect.

## Logs

Click the **Logs** header to expand the panel and see the last couple
hundred lines from the sing-box process — useful for diagnosing a failed
connection (see [06 — Troubleshooting](06-troubleshooting.md)). It refreshes
automatically every 2 seconds while open.

## The system tray

Wisp keeps a **tray icon** running whenever the app is open, with a
right-click menu:

- **Connect** / **Disconnect** — start or stop the tunnel without opening
  the window.
- **Show Wisp** — bring the main window back (also works by left-clicking
  the tray icon).
- **Quit** — actually exits the app (and disconnects the tunnel, since
  sing-box is a child process of Wisp).

> **Important:** Closing the Wisp window with the **X** button does **not**
> quit the app or disconnect your tunnel — it just hides the window to the
> tray. To fully exit, use **Quit** from the tray menu.

## Autostart (launch at login)

In the **Settings** card, check **Launch at login** and click **Save
settings** to have Wisp start automatically the next time you log into
Windows (you'll still see the elevation prompt described in
[01 — Installation](01-installation.md), since it's required every launch).
See [05 — Settings and MTU](05-settings-and-mtu.md) for the rest of the
settings.

---

Next: [04 — Split tunneling](04-split-tunneling.md), the headline feature.
