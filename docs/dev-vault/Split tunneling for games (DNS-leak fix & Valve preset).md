# Split tunneling for games — DNS-leak fix & Valve preset

Related: [[Split Tunneling]], [[sing-box Config Model]], [[Crate - wisp-core]],
[[Bug - Stale app binary on in-place upgrade]], [[Home]].

## The problem (real debugging session, 2026-07-02)
User excluded `dota2.exe` + Steam processes (Blacklist mode) but Dota 2 still
"could not determine latency". The tunnel WAS connected (v0.1.1). Log analysis
(`%LOCALAPPDATA%/Wisp/logs/`) showed:

- **872 flows → proxy, only 2 → direct**: the process-based exclude was barely
  routing anything direct.
- The game's DNS was resolved **through the tunnel** — `p2p-waw1.discovery.steamserver.net`
  (Warsaw) — so Steam discovered relays near the Bulgaria exit, not near the user.

## Two root causes
1. **Excluded apps' DNS was still hijacked/proxied.** The route rules put
   `hijack-dns` *before* the exclude rules, so excluded apps' port-53 traffic hit
   the DNS hijack first. → Game discovers wrong-region relays.
2. **sing-box process-matching is unreliable for the game's UDP** (short-lived
   Steam Datagram Relay probes), so they leaked to `final: proxy`.

## Fixes
1. **Rule reordering** (`wisp-core/src/singbox.rs`, `build_route`): in **Blacklist**
   mode the excluded rules now come **BEFORE** `hijack-dns`, so excluded apps are
   fully direct *including DNS*. (Whitelist keeps hijack-dns first.) Validated with
   `sing-box check` for Off/Blacklist/Whitelist.
2. **IP-range excludes + one-click Valve preset** — the reliable lever, since it
   doesn't depend on process matching. `wisp_core::valve_gaming_preset()`
   (`presets.rs`) returns:
   - **44 IPv4 + 32 IPv6** `IpCidr` rules = Valve's announced networks (**AS32590**),
   - Steam/Valve `DomainSuffix` rules (steampowered.com, dota2.com, counter-strike.net, …).
   The SDR relay range seen in the user's log (`155.133.230.0/24`) is included.
   Exposed in the UI as **"Exclude Valve / Steam games"** (adds the preset to the
   blacklist + reconnects).

### Refreshing the Valve ranges
Ranges came from RIPEstat announced-prefixes for AS32590:
`https://stat.ripe.net/data/announced-prefixes/data.json?resource=AS32590`
(fetched 2026-07-02). Re-fetch and regenerate `presets.rs` if Valve re-announces.

## Also shipped alongside
- `SplitMode` renamed **Exclude→Blacklist, Include→Whitelist** (serde aliases keep
  old `split.json` working).
- New rule kinds **`DomainRegex`→`domain_regex`**, **`ProcessPathRegex`→`process_path_regex`**
  (both validated accepted by the engine).
- Configurable **`log_level`** in `BuildSettings`/Settings (set `debug`/`trace` to
  see per-connection routing when diagnosing leaks).
- Split/settings changes now **force a live reconnect** so they apply immediately.

## Update (2026-07-02): the REAL root cause — DNS resolver path

v0.1.2 shipped the reordering + preset, user retested, **still broke**. Read the
user's actual log (`%LOCALAPPDATA%/Wisp/logs/`, readable from WSL at
`/mnt/c/Users/Semyon/AppData/...`) — the routing was already correct:

- **61 outbound connections, all `outbound/direct`, zero proxied.** The IP-CIDR
  preset worked: game HTTPS to `155.133.252.x` / `162.254.197.x` went direct.
- **But every DNS lookup took 10–37 seconds** (`dns: exchanged A ... [36.9s]`,
  `[30.74s]`, `[29.36s]`...). All DNS went to `dns-remote` (8.8.8.8 DoT,
  `detour: proxy`) — i.e. through the xhttp/REALITY tunnel to Bulgaria. Steam's
  relay-latency negotiation times out long before a 30 s answer → "cannot
  determine latency". Also `p2p-waw1.discovery.steamserver.net` (Warsaw)
  confirmed the resolver was geolocated to the exit.

**The gap:** the earlier fix corrected *route* ordering (excluded traffic goes
direct) but the config had **no `dns.rules`** — so excluded domains still
resolved via the slow proxied resolver.

**Fix (v0.1.3, `singbox.rs::build_dns`):** direct-routed domains/apps now resolve
via `dns-local`.
- sing-box resolves an unmatched query with the **first** server in the list, so
  server order encodes the default resolver per mode.
- **Blacklist**: servers `[dns-remote, dns-local]` (default = proxy); excluded
  domain/process rules pinned to `dns-local`.
- **Whitelist**: servers `[dns-local, dns-remote]` (default = local); only
  whitelisted domain/process rules pinned to `dns-remote`.
- `ip_cidr` rules are skipped for DNS (can't match a query by unresolved dest IP).
Validated Off/Blacklist/Whitelist with `sing-box check`; `process_name` IS
accepted in dns rules by the fork.

## Also (2026-07-02): in-place-upgrade bug is worse than noted
The stale-binary bug ([[Bug - Stale app binary on in-place upgrade]]) requires
killing **both** `wisp-app.exe` AND `sing-box.exe` before reinstall — the running
engine also locks files. The eventual installer hook (task #16) must terminate
both processes.

## Still uncertain (needs on-machine verification)
Whether process-matching alone ever reliably catches the game's UDP on Windows is
unproven — the IP-range preset is the robust path. Confirm from the user's log
(with `log_level=debug`) that game DNS now shows sub-second lookups and
`outbound/direct` after applying the preset on v0.1.3.
