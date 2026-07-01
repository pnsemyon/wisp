# Adding a Protocol or Transport

#guide #wisp-core

Because Wisp **passes through sing-box outbound JSON almost verbatim** (see
[[sing-box Config Model]] and [[Crate - wisp-core]]), adding support for a new protocol,
transport, or share-link format is mostly about **parsing** — teaching `wisp-core::parse.rs` to
produce the right outbound JSON shape — plus making sure `build_config` doesn't accidentally
filter it out. This note is a practical, checklist-driven guide.

## Where the work happens

| Concern | File |
|---|---|
| Recognizing supported outbound `"type"`s from raw JSON import | `crates/wisp-core/src/parse.rs` — `SUPPORTED_TYPES` |
| Parsing a new share-link scheme (`foo://...`) | `crates/wisp-core/src/parse.rs` — new `parse_foo_link()` function |
| Making sure the new outbound reaches the generated config | `crates/wisp-core/src/singbox.rs` — usually **nothing to change**, since `profile.outbounds` is copied verbatim |
| CLI support for testing it | Usually nothing — `wisp-cli`'s `parse`/`gen` work on any `Profile` — see [[Crate - wisp-cli]] |
| UI awareness (if the protocol needs its own UI fields) | `ui/src/main.js`/`index.html` — usually nothing, since the UI only reads generic fields (`tag`, `outbounds`) — see [[Web UI]] |

## Two kinds of "adding a protocol"

### 1. sing-box already supports it, Wisp's JSON importer doesn't recognize it yet

This is the common case: sing-box supports many outbound types (`trojan`, `shadowsocks`,
`vmess`, `wireguard`, ...), but `parse.rs`'s `import_json()` filters raw JSON import to
`SUPPORTED_TYPES`. If a user pastes a full sing-box config containing an outbound type not in
that list, it's silently dropped (`import_json` filters, doesn't error, unless *nothing* survives
the filter).

**Checklist:**
1. Add the new `"type"` string to `SUPPORTED_TYPES` in `parse.rs`.
2. Add a test mirroring `filters_unsupported_outbound_types` (see [[Crate - wisp-core]]) but
   proving the new type is now *kept* rather than filtered.
3. Nothing in `singbox.rs` needs to change — `build_config` copies `profile.outbounds`
   unmodified regardless of type (see [[sing-box Config Model#How the user's 3 outbounds map
   in|how outbounds map in]]).

### 2. Adding a new share-link scheme (e.g. `vmess://`, `trojan://`, `ss://`)

Follow the shape of `parse_vless_link`/`parse_hysteria2_link` in `parse.rs`:

**Checklist:**
1. Write `parse_<protocol>_link(link: &str) -> Result<Value>`:
   - `Url::parse(link)?`, check `url.scheme()` matches (support scheme aliases if the ecosystem
     has them, like `hysteria2://`/`hy2://`).
   - Extract required fields (host, port, credential) via `required_host`/`required_port`
     helpers (reuse them) and return a `WispError::Parse` if something mandatory is missing —
     see `crates/wisp-core/src/error.rs`.
   - Read `query_map(&url)` for optional params (TLS/SNI/fingerprint/transport-specific fields)
     and build the outbound `serde_json::Map` field-by-field, matching sing-box's own JSON
     schema for that outbound type. Cross-check field names against sing-box's own
     [configuration reference](https://sing-box.sagernet.org/configuration/outbound/) — Wisp
     doesn't validate these against a schema, so a typo'd field name will silently produce an
     outbound sing-box rejects at runtime (surfaced only when `sing-box run` fails — see
     [[Crate - wisp-engine]]'s health check in `wait_for_health`).
   - Use `link_name(&url)` for the display name (from the URL fragment) and
     `percent_decode` if needed (already used for fragments, which `url::Url` doesn't decode
     automatically).
2. Wire it into `import_links()`'s `if/else` chain by scheme prefix.
3. Add the outbound's `"type"` to `SUPPORTED_TYPES` too, if you want JSON import of that type to
   also work (usually yes, for consistency).
4. **Tests to add** (mirror the existing `vless`/`hysteria2` tests in `parse.rs`):
   - A "parses a full link into the expected outbound JSON" test, checking each field
     individually (see `parses_vless_reality_link_equivalent_to_fixture_outbound`).
   - A "scheme alias" test if applicable (see `hy2_scheme_alias_works`).
   - Consider adding an equivalent outbound to `REAL_CONFIG_FIXTURE` if the new protocol is
     significant enough to want coverage in `singbox.rs`'s `build_config` tests too — but keep
     it **synthetic/example data**, per [[Contributing#Security: never commit real server
     credentials|the security note]].
5. Run `cargo test -p wisp-core` (see [[Building and Running]]) and `cargo clippy -p wisp-core -- -D
   warnings` (see [[Contributing]]) before opening a PR.

## Adding a new transport (e.g. a sing-box transport type other than XHTTP)

The existing precedent is XHTTP support inside `parse_vless_link`: it only kicks in when
`params.get("type") == Some("xhttp")`, building a `transport: { type, path, ... }` sub-object.
For a new transport:

1. Detect it the same way (a `type=<transport>` query param, or whatever the share-link
   convention for that transport is).
2. Build the `transport` object matching sing-box's schema for that transport type.
3. Test it the same way as the XHTTP case (`outbound["transport"]["type"]`, etc. — see
   `parses_vless_reality_link_equivalent_to_fixture_outbound`).

## What you should *not* need to touch

- `wisp-engine` ([[Crate - wisp-engine]]) — it runs whatever config it's given; it has no
  protocol-specific logic at all.
- `src-tauri`/`ui` ([[Tauri Backend]], [[Web UI]]) — both operate on `Profile`/outbound JSON
  generically (tags, outbound arrays), with no protocol-specific branching.
- `singbox.rs`'s `build_config` — outbounds pass through verbatim; you'd only touch this file
  if you were changing something structural like the `selector`/`route` shape itself, which is
  a different kind of change (see [[Split Tunneling]], [[sing-box Config Model]]).

## See also

- [[Crate - wisp-core]] — the module this guide is entirely about.
- [[sing-box Config Model]] — why outbounds pass through unmodified.
- [[Glossary]] — REALITY, XHTTP, Vision, Hysteria2 definitions for context on the existing
  parsers you'll be mirroring.
- [[Contributing]] — conventions (tests, clippy, no real credentials in fixtures).
