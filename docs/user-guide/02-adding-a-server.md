# 02 — Adding a server

Wisp calls one imported connection a **profile**. A profile can contain one
server or several (see [Multi-server profiles](#multi-server-profiles-one-paste-many-servers)
below). To add one:

1. Click **Import** in the **Profile** card.
2. Paste one of the three supported formats (below) into the box.
3. Click **Import**.

If the paste is valid, the new profile is added, selected as active, and its
server(s) show up in the **Server** dropdown underneath the profile picker.
If something's wrong, an error message appears in the dialog (e.g. "share
link missing host") — fix the paste and try again.

> **Note:** All examples on this page use fake, placeholder hosts, UUIDs and
> passwords (`203.0.113.0/24` and `example.com` are reserved for
> documentation, never real addresses). Replace them with the real values
> your server or provider gave you — never reuse the example values as-is.

## Method 1 — a `vless://` share link

Use this for **VLESS + REALITY** (optionally with **XHTTP** transport) or
**VLESS + Vision** servers. Paste the whole link, for example:

**VLESS + REALITY + XHTTP:**

```
vless://11111111-2222-3333-4444-555555555555@203.0.113.10:38563?security=reality&pbk=ExamplePublicKeyAAAAAAAAAAAAAAAAAAAAAAAAAAA&sid=0123456789abcd&sni=www.example.com&fp=chrome&type=xhttp&path=%2F#My-Server-Reality
```

**VLESS + Vision (REALITY, no XHTTP, with `flow`):**

```
vless://11111111-2222-3333-4444-555555555555@203.0.113.10:37381?security=reality&flow=xtls-rprx-vision&pbk=ExamplePublicKeyBBBBBBBBBBBBBBBBBBBBBBBBBBB&sid=abcdef&sni=www.example.org&fp=firefox#My-Server-Vision
```

The bit after `#` (`My-Server-Reality`) becomes the server's display name.
Wisp reads the UUID, host, port, and the `security` / `pbk` (public key) /
`sid` (short ID) / `sni` / `fp` (fingerprint) / `flow` / `type=xhttp` /
`path` query parameters straight out of the link — whatever your provider
gave you should just paste in and work.

## Method 2 — a `hysteria2://` (or `hy2://`) link

Use this for **Hysteria2** servers:

```
hysteria2://example-password-1234@203.0.113.10:56085?obfs=salamander&obfs-password=example-obfs-pass-5678&sni=203.0.113.10&insecure=1#My-Server-Hysteria
```

`hy2://` is accepted as an alias of `hysteria2://`. Wisp reads the password,
host, port, optional `obfs`/`obfs-password` (traffic obfuscation), `sni`, and
`insecure` (skip TLS certificate verification — only use this if your
provider specifically tells you to).

## Method 3 — a full sing-box JSON config

If your provider gives you an entire sing-box config instead of a link,
paste the whole thing (or just the `outbounds` array). Wisp keeps every
outbound whose `"type"` it recognizes and ignores the rest (so you can paste
a full client config and Wisp will pick out just the proxy servers):

```json
{
  "outbounds": [
    {
      "type": "vless",
      "tag": "Example Location, Reality",
      "server": "203.0.113.10",
      "server_port": 38563,
      "uuid": "11111111-2222-3333-4444-555555555555",
      "tls": {
        "enabled": true,
        "server_name": "www.example.com",
        "utls": { "enabled": true, "fingerprint": "chrome" },
        "reality": {
          "enabled": true,
          "public_key": "ExamplePublicKeyAAAAAAAAAAAAAAAAAAAAAAAAAAA",
          "short_id": "0123456789abcd"
        }
      },
      "transport": { "type": "xhttp", "mode": "auto", "path": "/" }
    },
    {
      "type": "hysteria2",
      "tag": "Example Location, Hysteria",
      "server": "203.0.113.10",
      "server_port": 56085,
      "password": "example-password-1234",
      "obfs": { "type": "salamander", "password": "example-obfs-pass-5678" },
      "tls": { "enabled": true, "server_name": "203.0.113.10" }
    }
  ]
}
```

Each outbound's `"tag"` becomes its name in the **Server** dropdown; the
profile's own name comes from the first outbound's tag (with any trailing
`§ N` counter — added by some export tools — stripped off).

> **Note:** Wisp's JSON importer also recognizes `trojan`, `shadowsocks`, and
> `vmess` outbound types if they're present in a pasted config (so importing
> a mixed export doesn't error out), but Wisp's UI and testing are built
> around **VLESS + REALITY/Vision** and **Hysteria2** — those are the
> protocols to rely on.

## Multi-server profiles (one paste, many servers)

- **Multiple links:** paste several `vless://`/`hysteria2://` links into the
  Import box, **one per line** — each becomes a separate server inside the
  *same* profile. The profile's name is taken from the first link.
- **Multiple outbounds in one JSON:** a sing-box config with several
  supported outbounds (like the two-server example above) similarly becomes
  one profile with multiple servers.

Either way, after importing, use the **Server** dropdown (under the profile
picker) to pick which server in that profile is active. Switching servers
while connected takes effect immediately without reconnecting; it also
changes which server is used the next time you press Connect.

## Switching the active profile

If you've imported more than one profile, use the **Profile** dropdown to
pick which one is active. Wisp remembers your choice across restarts.

## Deleting a profile

1. Select the profile you want to remove in the **Profile** dropdown.
2. Click **Delete** (next to Import).
3. Confirm the prompt.

If the profile you delete was the active one, Wisp clears the active
selection — pick another profile (or import a new one) before connecting
again.

---

Next: [03 — Connecting](03-connecting.md).
