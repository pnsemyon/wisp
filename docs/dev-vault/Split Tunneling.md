# Split Tunneling

#split-tunneling #singbox

Split tunneling is one of Wisp's two headline features (the other is automatic
[[Glossary#MTU|MTU]] handling — see [[Architecture Overview]]). This note covers the data model
in [[Crate - wisp-core]]'s `split.rs`, and exactly how it's compiled into sing-box `route.rules`
by `singbox.rs`'s `build_route()`. For the surrounding config, see
[[sing-box Config Model]]; for the UI, see [[Web UI]].

## The model

```rust
pub enum SplitMode {
    Off,      // everything through the proxy
    Exclude,  // listed rules go direct, everything else proxied
    Include,  // only listed rules proxied, everything else direct
}

pub enum SplitRule {
    Process(String),      // e.g. "chrome.exe"
    ProcessPath(String),  // e.g. "C:\\Program Files\\...\\chrome.exe"
    DomainSuffix(String), // e.g. "example.com"
    IpCidr(String),       // e.g. "10.0.0.0/8"
}

pub struct SplitConfig { pub mode: SplitMode, pub rules: Vec<SplitRule> }
```

`SplitRule::field()` maps each variant to the sing-box route-rule field name it becomes:

| `SplitRule` variant | sing-box field |
|---|---|
| `Process` | `process_name` |
| `ProcessPath` | `process_path` |
| `DomainSuffix` | `domain_suffix` |
| `IpCidr` | `ip_cidr` |

## Exactly how it becomes `route.rules`

`build_route(split)` in `singbox.rs` always starts with three fixed rules, in order:

1. `{"action": "sniff"}`
2. `{"protocol": "dns", "action": "hijack-dns"}`
3. `{"ip_is_private": true, "outbound": "direct"}`

Then, depending on `split.mode`:

- **`Off`**: no further rules are added. `route.final = "proxy"`. (Everything not private-IP
  goes through the tunnel.)
- **`Exclude`**: `rule_group(&split.rules, "direct")` appends one rule *per distinct field*,
  each routing matching traffic to `"direct"`. `route.final = "proxy"`. (Listed items bypass
  the tunnel; everything else — the default — goes through it.)
- **`Include`**: `rule_group(&split.rules, "proxy")` appends one rule per distinct field, each
  routing matching traffic to `"proxy"`. `route.final = "direct"`. (Only listed items use the
  tunnel; everything else goes direct.)

`rule_group` groups rules by field name using a `BTreeMap<&str, Vec<String>>` — so **multiple
rules of the same kind become one sing-box rule with an array of values**, not N separate
rules. Field ordering in the output is alphabetical (`BTreeMap`'s natural order): `domain_suffix`
< `ip_cidr` < `process_name` < `process_path`.

## Worked example

Split config: `Exclude` mode, rules = `[Process("chrome.exe"), Process("firefox.exe"),
DomainSuffix("example.com")]`.

Resulting `route`:

```json
{
  "auto_detect_interface": true,
  "final": "proxy",
  "rules": [
    { "action": "sniff" },
    { "protocol": "dns", "action": "hijack-dns" },
    { "ip_is_private": true, "outbound": "direct" },
    { "domain_suffix": ["example.com"], "outbound": "direct" },
    { "process_name": ["chrome.exe", "firefox.exe"], "outbound": "direct" }
  ]
}
```

Reading this top to bottom: sniff and DNS-hijack always happen first; private-IP/LAN traffic
always goes direct; then `example.com` (and subdomains, since it's a *suffix* match) goes
direct; then both listed processes go direct; anything else falls through to `route.final`,
which is `"proxy"` — so Chrome, Firefox, and traffic to `example.com` bypass the tunnel, while
every other app/domain is proxied.

If the mode were `Include` instead (same rules), the last two generated rules would route to
`"proxy"` instead of `"direct"`, and `route.final` would be `"direct"` — so *only* Chrome,
Firefox, and `example.com` traffic would use the tunnel, and everything else would go direct.

## Where this is exercised

- `wisp-core`'s `singbox.rs` tests assert this behavior directly for both modes and for the
  grouping logic (`split_exclude_routes_chrome_direct_and_final_proxy`,
  `split_include_routes_chrome_proxy_and_final_direct`,
  `multiple_rules_of_same_field_are_grouped`) — see [[Crate - wisp-core]].
- `wisp-cli`'s `gen`/`run` subcommands accept `--mode` and repeated `--rule kind:value` flags
  that build a `SplitConfig` the same way — see [[Crate - wisp-cli]].
- The Tauri backend's `get_split`/`set_split_mode`/`add_split_rule`/`remove_split_rule`
  commands persist a `SplitConfig` to `split.json` and feed it to `build_config` on every
  `connect()` — see [[Tauri Backend]].
- The [[Web UI]]'s Split tunneling card is the human-facing editor for this model: mode radio
  buttons, an "Add app" picker backed by `list_running_processes`, and a domain text input.

## See also

- [[Crate - wisp-core]] — `split.rs` and `singbox.rs::build_route`.
- [[sing-box Config Model]] — the full config this logic is embedded in.
- [[Tauri Backend]] — the commands that mutate/persist `SplitConfig`.
- [[Web UI]] — the split-tunnel UI panel.
- [[Crate - wisp-cli]] — `--mode`/`--rule` CLI flags.
