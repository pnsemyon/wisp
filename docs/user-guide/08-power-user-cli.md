# 08 — Power-user CLI (`wisp`)

`wisp-cli` is a headless command-line tool built on the same `wisp-core` /
`wisp-engine` code the desktop app uses. It's meant for testing configs and
scripting/headless use — not a replacement for the GUI's split-tunnel picker
or tray.

```
wisp <COMMAND>
```

Build it from the repo root:

```bash
cargo build -p wisp-cli --release
# binary at target/release/wisp (wisp.exe on Windows)
```

> **Note:** `parse` and `gen` are pure Rust and work on any OS (they only
> read a file and print JSON/text). `run` actually launches sing-box, so it
> needs the real `sing-box.exe`/Wintun setup and generally needs to run on
> Windows with admin rights for the TUN adapter to come up — same
> requirement as the desktop app (see
> [01 — Installation](01-installation.md)).

## `wisp parse <file>` — inspect a profile

Reads a sing-box JSON config or a file of share links (same formats as the
GUI's Import dialog, see [02 — Adding a server](02-adding-a-server.md)) and
prints a summary: profile name/id, every server ("outbound") tag, and which
one is active.

```bash
wisp parse my-server.txt
```

```
profile: my-server-reality (my-server-reality)
outbounds: 2
  - My-Server-Reality
  - My-Server-Hysteria
active_tag: My-Server-Reality
```

Where `my-server.txt` contains, e.g.:

```
vless://11111111-2222-3333-4444-555555555555@203.0.113.10:38563?security=reality&pbk=ExamplePublicKeyAAAAAAAAAAAAAAAAAAAAAAAAAAA&sid=0123456789abcd&sni=www.example.com&fp=chrome&type=xhttp&path=%2F#My-Server-Reality
hysteria2://example-password-1234@203.0.113.10:56085?obfs=salamander&obfs-password=example-obfs-pass-5678&sni=203.0.113.10&insecure=1#My-Server-Hysteria
```

## `wisp gen <file>` — generate the sing-box config

Builds the exact sing-box JSON config Wisp would use for a profile, split
mode, and MTU — useful for reviewing exactly what will be run, or feeding
into your own tooling.

```bash
wisp gen my-server.txt --mtu 1280 --mode exclude --rule process:chrome.exe --rule domain_suffix:netflix.com
```

Flags:

| Flag | Meaning |
|---|---|
| `--mtu <u32>` | TUN MTU (default `1280`, same default as the GUI). |
| `--mode <off\|exclude\|include>` | Split-tunnel mode (default `off`). See [04 — Split tunneling](04-split-tunneling.md). |
| `--rule <kind:value>` | A split-tunnel rule, repeatable. `kind` is one of `process`, `process_path`, `domain_suffix`, `ip_cidr`. |

Prints the full sing-box config as pretty-printed JSON to stdout.

## `wisp run <file>` — actually run sing-box

Imports the profile, builds the config (same flags as `gen`), locates the
bundled `sing-box.exe` (or one you point at with `--binary`), starts it, and
prints live traffic stats every 2 seconds until you press Ctrl-C — a
minimal headless equivalent of clicking **Connect** in the GUI.

```bash
wisp run my-server.txt --mode include --rule process:msedge.exe
```

```
sing-box started; printing stats every 2s, press Ctrl-C to stop
TrafficStats { up_bytes: 1024, down_bytes: 8192, up_speed: 512, down_speed: 4096 }
...
```

Flags: same `--mtu`/`--mode`/`--rule` as `gen`, plus:

| Flag | Meaning |
|---|---|
| `--binary <path>` | Path to `sing-box.exe`. Defaults to auto-detecting it next to the app (the same resource lookup the GUI uses — see [01 — Installation](01-installation.md#getting-the-app)). |

Press **Ctrl-C** to stop sing-box cleanly.

---

Back to the [user guide index](README.md).
