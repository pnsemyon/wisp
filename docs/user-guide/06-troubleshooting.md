# 06 — Troubleshooting

## Can't connect / no internet after connecting

| Cause | Fix |
|---|---|
| **MTU too high for your network path** | Open **Settings**, lower the MTU (try 1200), save, disconnect and reconnect. Symptoms: connects fine, status shows Running, but pages hang or load only partially. See [05 — Settings and MTU](05-settings-and-mtu.md#symptoms-of-a-wrong-mtu). |
| **Wisp isn't actually elevated** | Wisp needs admin rights to create the TUN adapter. If you clicked "No" on the UAC prompt, or a security tool blocked it, close Wisp and relaunch it, accepting the prompt. See [01 — Installation](01-installation.md#first-launch). |
| **Another VPN or TUN-based tool is already running** | Only one program can usually own the system's default route / TUN adapter cleanly. Disconnect/quit other VPN clients (or tools like other TUN-based proxies) before connecting with Wisp, then try again. |
| **`wintun.dll` is missing** | If Wisp can start sing-box but the TUN adapter never comes up, `wintun.dll` may not be next to the app. Re-run `scripts\fetch-resources.ps1` (see [01 — Installation](01-installation.md#getting-the-app)) and restart Wisp. |
| **Status stuck on "Errored"** | Open the **Logs** panel ([03 — Connecting](03-connecting.md#logs)) and read the last lines from sing-box — they usually name the exact problem (bad server, TLS handshake failure, port in use, etc.). |

## Elevation (UAC) prompt keeps appearing

This is expected — Wisp needs administrator rights every time it starts (to
create the TUN adapter), so it always requests elevation on launch, even if
you approved it last time. See
[01 — Installation](01-installation.md#first-launch) for why.

If it's popping up *more than once per launch*, or seems to loop:

- Make sure you're not launching Wisp from a shortcut that itself already
  runs elevated *and* a second copy that isn't — only run one instance.
- Check whether antivirus/endpoint software is interfering with the
  relaunch — some security tools block programs from relaunching
  themselves; check your AV's logs/quarantine.

## Some app still bypasses the tunnel — or gets tunneled when it shouldn't

| Cause | Fix |
|---|---|
| **Wrong split mode for what you want** | Re-check the table in [04 — Split tunneling](04-split-tunneling.md#decision-table--i-want-to). **Exclude** tunnels everything *except* your rules; **Include** tunnels *only* your rules. Mixing these up is the most common mistake. |
| **Rule added for the wrong process name** | App rules match the exact executable **name** (e.g. `chrome.exe`), not a friendly name or full path. Use **+ Add app** and pick the process from the running list instead of typing it, to avoid typos. |
| **Same executable name used by multiple programs** | Rules match by process name only, not by folder/path — if two different apps share an executable name, a rule affects both. There's currently no UI way to disambiguate by path. |
| **App wasn't running when you tried to add it** | The **+ Add app** picker only lists currently running processes. Start the app first, then add the rule. |
| **Domain rule doesn't cover the traffic you expected** | Domain rules match by **suffix** — `netflix.com` matches subdomains of `netflix.com`, but a service split across multiple unrelated domains/CDNs may need more than one domain rule added. |
| **Split-tunnel change made while connected didn't seem to apply** | Mode and rule changes take effect immediately in the running tunnel — if it still looks wrong, double check you edited the rule for the right app/domain (see the rule list in the Split tunneling card) rather than assuming it needs a reconnect. |

## DNS not resolving

- Confirm the tunnel status is actually **Running** (not Starting/Errored) —
  see [03 — Connecting](03-connecting.md#reading-the-status-pill).
- If only *some* domains fail to resolve, check whether you've added an
  **Include**-mode rule set that excludes the domain in question from the
  tunnel entirely, or an **Exclude** rule that's sending it (and its DNS)
  direct when your network can't reach it directly.
- If nothing resolves at all while Running, treat it as a connection
  problem — see [Can't connect / no internet after connecting](#cant-connect--no-internet-after-connecting)
  above (MTU is the most common cause).

## Slow speeds

- **Try a lower MTU first if speeds are inconsistent or connections stall
  partway through** — this can look like "slow" but is actually packets
  being dropped/retried. See [05 — Settings and MTU](05-settings-and-mtu.md).
- **Switch servers**, if your profile has more than one, using the **Server**
  dropdown — some servers/locations are simply faster than others.
- **Check the Logs panel** for repeated errors/reconnects, which usually
  indicate a struggling server or path rather than a Wisp-side problem.
- Split tunneling doesn't affect the speed of tunneled traffic — only which
  traffic is tunneled — so switching modes won't fix raw throughput issues.

## `sing-box.exe not found` (or similar resource errors)

Wisp bundles the sing-box engine binary alongside itself but doesn't ship it
in source control — you need to fetch it once:

```powershell
.\scripts\fetch-resources.ps1
```

This downloads the pinned `sing-box.exe` and `wintun.dll` into `resources\`
next to the app. Restart Wisp afterwards. See
[01 — Installation](01-installation.md#getting-the-app) for the full setup.

---

Still stuck? Check [07 — FAQ](07-faq.md), or open the **Logs** panel and
look for the specific error sing-box reports.
