# Glossary

#glossary #reference

Short definitions of terms used throughout this vault, cross-linked to the notes that go deeper
on each.

## sing-box

The proxy/VPN engine Wisp wraps instead of reimplementing. An open-source, actively-maintained
(Go) universal proxy platform supporting many protocols (VLESS, Hysteria2, Trojan, Shadowsocks,
VMess, WireGuard, ...) with a single config format and a `tun` inbound for system-wide capture.
Same engine used by Hiddify and the official sing-box apps. See
[[Architecture Overview#Why wrap sing-box instead of reimplementing?|why Wisp wraps it]] and
[[sing-box Config Model]] for the config it consumes.

## REALITY

A TLS-handshake-mimicry technique (part of the Xray/sing-box ecosystem) used with VLESS: instead
of terminating TLS at the proxy server, REALITY forwards the TLS handshake to a real, innocuous
website (the `server_name`/SNI, e.g. `www.amazon.com`) so a passive observer sees what looks
like a normal HTTPS connection to that site, while only clients with the correct key
(`public_key`/`pbk`, `short_id`/`sid`) can actually establish the proxied tunnel. See
[[sing-box Config Model]] for how it appears in a generated config
(`tls.reality.{enabled,public_key,short_id}`) and [[Crate - wisp-core]] for how it's parsed from
a `vless://` link's `security=reality` query params.

## XHTTP

A sing-box transport (carried inside the VLESS+REALITY connection) that shapes traffic to look
like ordinary HTTP request/response traffic, adding padding and chunking controls
(`xPaddingBytes`, `scMaxEachPostBytes` in the raw sing-box outbound) to further resist traffic
analysis. Detected in share links via `type=xhttp`. See [[Crate - wisp-core]]'s
`parse_vless_link`.

## Vision (xtls-rprx-vision)

A VLESS "flow" (`flow: "xtls-rprx-vision"`) — an XTLS variant that improves throughput and
adds padding/obfuscation at the TLS record level compared to plain VLESS, without a separate
transport layer. One of the two VLESS+REALITY outbound styles in Wisp's fixture profile (see
[[Crate - wisp-core]]) uses this flow; the other omits it.

## Hysteria2

A UDP-based (QUIC-like) proxy protocol built for high throughput and loss resilience,
independent of VLESS/REALITY. Uses a `password` for auth, optional `obfs` (traffic obfuscation,
e.g. `salamander`), and its own TLS block. Wisp parses `hysteria2://`/`hy2://` share links in
[[Crate - wisp-core]]'s `parse_hysteria2_link`.

## Wintun

The Windows driver (by the WireGuard project) that provides a fast, kernel-level virtual network
adapter without requiring a full TAP driver install/signing dance. sing-box uses it on Windows
to implement its `tun` inbound. Wisp bundles `wintun.dll` (fetched via
`scripts/fetch-resources.*`, located at runtime by [[Crate - wisp-engine]]'s
`locate_resources()`) rather than requiring users to install it separately. See
[[Building and Running]].

## TUN

A virtual network interface type that operates at the IP layer, letting a userspace program
(sing-box, here) see and inject raw IP packets. Wisp's generated config's single `tun` inbound
(tag `tun-in`) is what makes the app a *system-wide* VPN rather than a per-app SOCKS proxy — see
[[sing-box Config Model#`inbounds`: the `tun` entry|the `inbounds` section]]. Creating this
adapter is why Wisp needs [[Tauri Backend#Windows elevation — `elevation.rs`|admin elevation]]
on Windows.

## Clash API

sing-box's built-in Clash-compatible HTTP control API (`experimental.clash_api` in the config —
see [[sing-box Config Model]]), bound to `127.0.0.1:<port>` with an optional bearer secret.
[[Crate - wisp-engine]]'s `ClashApi` client uses it for exactly two things: `GET /connections`
(traffic totals, used to derive speed) and `PUT /proxies/<selector>` (switch the active
outbound). Named "Clash" because it mirrors the API shape of the (now largely superseded) Clash
proxy client, which many sing-box-ecosystem tools chose to stay compatible with.

## MTU

Maximum Transmission Unit — the largest IP packet size a network link will pass without
fragmentation. Tunnels typically need a *smaller* MTU on their virtual interface than the
underlying physical link (to leave room for the tunnel's own encapsulation overhead); getting
this wrong causes mysterious partial connectivity (small pages load, large ones hang). Wisp's
answer to "no more manual MTU fiddling" (per the README) is baking `mtu` directly into the
generated `tun` inbound (default **1280**) via `BuildSettings.mtu` — see
[[Crate - wisp-core]] and [[sing-box Config Model]] — instead of requiring a manual
`netsh interface ipv4 set subinterface ... mtu=...` step after connecting.

## Split tunneling

Routing only *some* traffic (chosen by app/process, domain, or IP range) through the VPN tunnel
while the rest goes direct — or the inverse. Wisp's take on this is the `SplitMode`
(`Off`/`Exclude`/`Include`) + `SplitRule`
(`Process`/`ProcessPath`/`DomainSuffix`/`IpCidr`) model in [[Crate - wisp-core]], compiled into
sing-box `route.rules`. Full mechanics and a worked example: [[Split Tunneling]].

## Selector

A sing-box outbound type (`"type": "selector"`) that acts as a named, switchable pointer to one
of several other outbounds — Wisp generates one (tagged `"proxy"`) listing all of a profile's
server outbounds, so route rules can target `"proxy"` once and switching servers at runtime is
just a `PUT /proxies/proxy` [[Glossary#Clash API|Clash API]] call rather than a config rebuild.
See [[sing-box Config Model#How the user's 3 outbounds map in|how the selector is built]] and
[[Crate - wisp-engine]]'s `switch()` method.

## See also

Every term above links back to the note that goes deeper on it. Start at [[Home]] if you landed
here directly, or [[Architecture Overview]] for the big picture these terms fit into.
