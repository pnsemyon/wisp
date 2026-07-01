//! Build a complete sing-box configuration from a `Profile` and
//! `SplitConfig`.
//!
//! Targets the sing-box 1.13 config schema: typed DNS servers, a
//! `default_domain_resolver` on `route`, and no `block` outbound type
//! (removed upstream in 1.12). The bundled engine is
//! `shtorm-7/sing-box-extended`, a fork of mainline sing-box 1.13 (identical
//! config schema otherwise) that adds Xray transports, including `xhttp`;
//! see [`normalize_xhttp_transport`] for the camelCase-vs-snake_case wrinkle
//! that transport requires. The older `splithttp` name is still not
//! supported and is dropped.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::error::{Result, WispError};
use crate::profile::Profile;
use crate::split::{SplitConfig, SplitMode, SplitRule};

/// Settings that control the shape of the generated config but aren't part
/// of the profile or split-tunnel model.
#[derive(Debug, Clone)]
pub struct BuildSettings {
    /// TUN interface MTU. Default 1280.
    pub mtu: u32,
    /// Secret for sing-box's Clash API.
    pub clash_secret: String,
    /// Port for sing-box's Clash API (`127.0.0.1:<port>`). Default 9090.
    pub clash_port: u16,
    /// Optional local SOCKS inbound port, in addition to the TUN inbound.
    pub socks_port: Option<u16>,
    /// sing-box `log.level`. Default `"info"`; the app can raise this to
    /// `"debug"`/`"trace"` for diagnosing routing issues.
    pub log_level: String,
}

impl Default for BuildSettings {
    fn default() -> Self {
        BuildSettings {
            mtu: 1280,
            clash_secret: String::new(),
            clash_port: 9090,
            socks_port: None,
            log_level: "info".to_string(),
        }
    }
}

/// `transport.type` values the bundled `sing-box-extended` engine supports.
/// `xhttp` is included because the bundled fork implements it (mainline
/// sing-box does not); outbounds using it are normalized by
/// [`normalize_xhttp_transport`] before being emitted. Anything not in this
/// list (notably the older Xray transport name `splithttp`, which the fork
/// doesn't recognize either) is dropped with a `tracing::warn!`.
const SUPPORTED_TRANSPORTS: &[&str] = &["http", "ws", "grpc", "httpupgrade", "quic", "xhttp"];

/// Whether `outbound` uses a transport the bundled sing-box engine can run:
/// no `transport` at all, or a `transport.type` in [`SUPPORTED_TRANSPORTS`].
fn is_supported_outbound(outbound: &Value) -> bool {
    match outbound.get("transport") {
        None => true,
        Some(transport) => transport
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|t| SUPPORTED_TRANSPORTS.contains(&t)),
    }
}

fn tag_of(outbound: &Value) -> Option<String> {
    outbound
        .get("tag")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

fn transport_type_of(outbound: &Value) -> String {
    outbound
        .get("transport")
        .and_then(|t| t.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string()
}

/// Split `outbounds` into the ones the bundled engine supports and a list of
/// `(tag, transport_type)` pairs for the ones that were skipped.
fn filter_supported_outbounds(outbounds: &[Value]) -> (Vec<Value>, Vec<(String, String)>) {
    let mut supported = Vec::with_capacity(outbounds.len());
    let mut skipped = Vec::new();
    for outbound in outbounds {
        if is_supported_outbound(outbound) {
            supported.push(outbound.clone());
        } else {
            let tag = tag_of(outbound).unwrap_or_else(|| "<untagged>".to_string());
            skipped.push((tag, transport_type_of(outbound)));
        }
    }
    (supported, skipped)
}

/// Rename a camelCase identifier to snake_case: an underscore is inserted
/// before each run of uppercase letters (except at the very start), and the
/// whole thing is lowercased. Keys that are already snake_case (no
/// uppercase letters) round-trip unchanged.
///
/// `xPaddingBytes` -> `x_padding_bytes`, `scMaxEachPostBytes` ->
/// `sc_max_each_post_bytes`, `path` -> `path`.
fn camel_to_snake(key: &str) -> String {
    let mut out = String::with_capacity(key.len() + 4);
    let mut prev_upper = false;
    for (i, ch) in key.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 && !prev_upper {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_upper = true;
        } else {
            out.push(ch);
            prev_upper = false;
        }
    }
    out
}

/// Normalize an `xhttp` `transport` object in place for the bundled
/// `sing-box-extended` fork:
///
/// - Every key is renamed from camelCase to snake_case (see
///   [`camel_to_snake`]), preserving values. This is needed because Xray
///   (and share links / configs copied from Xray-based clients) use
///   camelCase field names like `xPaddingBytes` and `scMaxEachPostBytes`,
///   but the fork's xhttp implementation only recognizes the snake_case
///   forms and silently ignores anything else.
/// - After renaming, if `x_padding_bytes` is still absent, or present but
///   empty/`"0"`, a standard anti-detection default (`"100-1000"`) is
///   injected. The fork requires non-zero padding on xhttp transports and
///   fails to start without it.
///
/// No-op if `transport` isn't a JSON object.
fn normalize_xhttp_transport(transport: &mut Value) {
    let Some(obj) = transport.as_object_mut() else {
        return;
    };

    let renamed: Vec<(String, Value)> = obj
        .iter()
        .map(|(k, v)| (camel_to_snake(k), v.clone()))
        .collect();
    obj.clear();
    for (key, value) in renamed {
        obj.insert(key, value);
    }

    let needs_default = match obj.get("x_padding_bytes") {
        None | Some(Value::Null) => true,
        Some(Value::String(s)) => s.is_empty() || s == "0",
        _ => false,
    };
    if needs_default {
        obj.insert(
            "x_padding_bytes".to_string(),
            Value::String("100-1000".to_string()),
        );
    }
}

/// Build a complete, ready-to-run sing-box config for `profile`, applying
/// `split` tunneling rules and `settings`.
///
/// Outbounds whose transport isn't supported by the bundled engine (e.g.
/// `splithttp`) are dropped; see [`is_supported_outbound`]. If that leaves
/// zero outbounds, this returns an error rather than emitting a config with
/// an empty selector. `xhttp` outbounds are supported and are normalized
/// in place (camelCase Xray field names -> the fork's snake_case, plus a
/// default padding value) by [`normalize_xhttp_transport`].
pub fn build_config(
    profile: &Profile,
    split: &SplitConfig,
    settings: &BuildSettings,
) -> Result<Value> {
    tracing::debug!(
        mtu = settings.mtu,
        clash_port = settings.clash_port,
        split_mode = ?split.mode,
        rule_count = split.rules.len(),
        outbound_count = profile.outbounds.len(),
        "build_config: building sing-box config"
    );

    let (mut outbounds, skipped) = filter_supported_outbounds(&profile.outbounds);
    for (tag, transport) in &skipped {
        tracing::warn!(
            tag = %tag,
            transport = %transport,
            "build_config: skipping outbound with unsupported transport"
        );
    }
    if outbounds.is_empty() {
        let mut transports: Vec<String> = skipped.into_iter().map(|(_, t)| t).collect();
        transports.sort();
        transports.dedup();
        return Err(WispError::Other(format!(
            "No outbounds are supported by the bundled sing-box engine (unsupported transports: {}). \
             Use a Vision/Hysteria2/ws/grpc server instead.",
            transports.join(", ")
        )));
    }

    for outbound in outbounds.iter_mut() {
        let is_xhttp = outbound
            .get("transport")
            .and_then(|t| t.get("type"))
            .and_then(Value::as_str)
            == Some("xhttp");
        if is_xhttp {
            if let Some(transport) = outbound.get_mut("transport") {
                normalize_xhttp_transport(transport);
            }
        }
    }

    let tags: Vec<String> = outbounds.iter().filter_map(tag_of).collect();
    tracing::debug!(
        selector_tags = tags.len(),
        "build_config: selector outbound tags"
    );
    let default_tag = profile
        .active_tag
        .clone()
        .filter(|t| tags.contains(t))
        .or_else(|| tags.first().cloned());

    outbounds.push(json!({
        "type": "selector",
        "tag": "proxy",
        "outbounds": tags,
        "default": default_tag,
    }));
    outbounds.push(json!({ "type": "direct", "tag": "direct" }));

    let mut inbounds = vec![json!({
        "type": "tun",
        "tag": "tun-in",
        "address": ["172.19.0.1/30"],
        "mtu": settings.mtu,
        "auto_route": true,
        "strict_route": true,
        "stack": "system",
    })];
    if let Some(port) = settings.socks_port {
        inbounds.push(json!({
            "type": "socks",
            "tag": "socks-in",
            "listen": "127.0.0.1",
            "listen_port": port,
        }));
    }

    let route = build_route(split);

    Ok(json!({
        "log": { "level": settings.log_level, "timestamp": true },
        "dns": {
            "servers": [
                { "type": "tls", "tag": "dns-remote", "server": "8.8.8.8", "detour": "proxy" },
                { "type": "local", "tag": "dns-local" }
            ],
            "strategy": "prefer_ipv4"
        },
        "inbounds": inbounds,
        "outbounds": outbounds,
        "route": route,
        "experimental": {
            "clash_api": {
                "external_controller": format!("127.0.0.1:{}", settings.clash_port),
                "secret": settings.clash_secret,
            }
        }
    }))
}

fn build_route(split: &SplitConfig) -> Value {
    let hijack_dns = json!({ "protocol": "dns", "action": "hijack-dns" });
    let private_direct = json!({ "ip_is_private": true, "outbound": "direct" });

    let (rules, final_outbound): (Vec<Value>, &'static str) = match split.mode {
        SplitMode::Off => (
            vec![json!({ "action": "sniff" }), hijack_dns, private_direct],
            "proxy",
        ),
        SplitMode::Blacklist => {
            // Excluded apps' rules must come BEFORE hijack-dns: otherwise
            // their DNS lookups are still hijacked/proxied even though
            // their traffic goes direct, which can make e.g. a game
            // discover a wrong-region relay while its traffic itself
            // bypasses the proxy. Putting the exclusion rules first makes
            // excluded apps fully direct, DNS included.
            let mut rules = vec![json!({ "action": "sniff" })];
            rules.extend(rule_group(&split.rules, "direct"));
            rules.push(hijack_dns);
            rules.push(private_direct);
            (rules, "proxy")
        }
        SplitMode::Whitelist => {
            let mut rules = vec![json!({ "action": "sniff" }), hijack_dns, private_direct];
            rules.extend(rule_group(&split.rules, "proxy"));
            (rules, "direct")
        }
    };

    json!({
        "auto_detect_interface": true,
        "default_domain_resolver": "dns-local",
        "final": final_outbound,
        "rules": rules,
    })
}

/// Group `rules` by their sing-box field name (e.g. all `process_name`
/// entries share one rule with an array of values) and emit one route rule
/// per field, all pointing at `outbound`.
fn rule_group(rules: &[SplitRule], outbound: &str) -> Vec<Value> {
    let mut grouped: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for rule in rules {
        let (field, value) = rule.field();
        grouped.entry(field).or_default().push(value.to_string());
    }

    grouped
        .into_iter()
        .map(|(field, values)| {
            json!({
                field: values,
                "outbound": outbound,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::REAL_CONFIG_FIXTURE;
    use crate::split::SplitConfig;
    use std::io::Write;
    use std::process::Command;

    fn fixture_profile() -> Profile {
        crate::parse::import(REAL_CONFIG_FIXTURE).expect("fixture should import")
    }

    /// Tag of fixture outbound 0: vless+xhttp (supported by the bundled fork).
    const XHTTP_TAG: &str = "Bulgaria, Sophia-7w1t0rtt5a § 0";
    /// Tag of fixture outbound 1: vless+Vision, no transport (supported).
    const VISION_TAG: &str = "Bulgaria, Sophia-7w1t0rtt5a § 1";
    /// Tag of fixture outbound 2: hysteria2 (supported).
    const HYSTERIA2_TAG: &str = "Bulgaria, Sophia, hysteria-7w1t0rtt5a § 2";

    #[test]
    fn has_one_tun_inbound_with_configured_mtu() {
        let profile = fixture_profile();
        let split = SplitConfig::default();
        let settings = BuildSettings {
            mtu: 1280,
            ..BuildSettings::default()
        };
        let config = build_config(&profile, &split, &settings).expect("build should succeed");

        let inbounds = config["inbounds"].as_array().expect("inbounds array");
        let tun_inbounds: Vec<&Value> = inbounds.iter().filter(|i| i["type"] == "tun").collect();
        assert_eq!(tun_inbounds.len(), 1);
        assert_eq!(tun_inbounds[0]["mtu"], 1280);
        assert_eq!(tun_inbounds[0]["auto_route"], true);
        assert_eq!(tun_inbounds[0]["strict_route"], true);
        assert_eq!(tun_inbounds[0]["stack"], "system");
        assert_eq!(tun_inbounds[0]["address"], json!(["172.19.0.1/30"]));
        // sing-box 1.13 dropped this field from the tun inbound schema.
        assert!(tun_inbounds[0].get("endpoint_independent_nat").is_none());
    }

    #[test]
    fn dns_uses_new_typed_server_schema() {
        let profile = fixture_profile();
        let split = SplitConfig::default();
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");

        let servers = config["dns"]["servers"].as_array().expect("servers array");
        assert_eq!(servers.len(), 2);

        let remote = servers
            .iter()
            .find(|s| s["tag"] == "dns-remote")
            .expect("dns-remote present");
        assert_eq!(remote["type"], "tls");
        assert_eq!(remote["server"], "8.8.8.8");
        assert_eq!(remote["detour"], "proxy");
        // The legacy `address` field is gone in the new schema.
        assert!(remote.get("address").is_none());

        let local = servers
            .iter()
            .find(|s| s["tag"] == "dns-local")
            .expect("dns-local present");
        assert_eq!(local["type"], "local");
        assert!(local.get("address").is_none());

        assert_eq!(config["dns"]["strategy"], "prefer_ipv4");
    }

    #[test]
    fn route_has_default_domain_resolver() {
        let profile = fixture_profile();
        let split = SplitConfig::default();
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");

        assert_eq!(config["route"]["default_domain_resolver"], "dns-local");
        assert_eq!(config["route"]["auto_detect_interface"], true);
    }

    #[test]
    fn no_block_outbound_is_ever_emitted() {
        let profile = fixture_profile();
        let split = SplitConfig::default();
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");

        let outbounds = config["outbounds"].as_array().expect("outbounds array");
        assert!(
            !outbounds.iter().any(|o| o["type"] == "block"),
            "block outbound type was removed in sing-box 1.12 and must never be emitted"
        );
    }

    #[test]
    fn xhttp_vision_and_hysteria2_all_kept_and_xhttp_normalized() {
        let profile = fixture_profile();
        let split = SplitConfig::default();
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");

        let outbounds = config["outbounds"].as_array().expect("outbounds array");
        assert!(
            outbounds.iter().any(|o| o["tag"] == XHTTP_TAG),
            "xhttp outbound is supported by the bundled fork and must be kept"
        );
        assert!(outbounds.iter().any(|o| o["tag"] == VISION_TAG));
        assert!(outbounds.iter().any(|o| o["tag"] == HYSTERIA2_TAG));

        let xhttp_outbound = outbounds
            .iter()
            .find(|o| o["tag"] == XHTTP_TAG)
            .expect("xhttp outbound present");
        let transport = &xhttp_outbound["transport"];
        assert_eq!(transport["type"], "xhttp");
        // The fixture supplies Xray camelCase field names; build_config must
        // rename them to the snake_case the bundled fork requires, with
        // values preserved.
        assert_eq!(transport["x_padding_bytes"], "100-1000");
        assert_eq!(transport["sc_max_each_post_bytes"], "1000000-1000000");
        assert!(transport.get("xPaddingBytes").is_none());
        assert!(transport.get("scMaxEachPostBytes").is_none());

        let selector = outbounds
            .iter()
            .find(|o| o["type"] == "selector" && o["tag"] == "proxy")
            .expect("selector outbound present");
        let selector_tags: Vec<String> = selector["outbounds"]
            .as_array()
            .expect("selector outbounds array")
            .iter()
            .map(|v| v.as_str().expect("tag is string").to_string())
            .collect();
        assert_eq!(selector_tags.len(), 3);
        assert!(selector_tags.contains(&XHTTP_TAG.to_string()));
        assert!(selector_tags.contains(&VISION_TAG.to_string()));
        assert!(selector_tags.contains(&HYSTERIA2_TAG.to_string()));
    }

    #[test]
    fn selector_default_prefers_active_tag_now_that_xhttp_is_supported() {
        // The fixture's active_tag is the xhttp outbound (first in the
        // list). Now that xhttp is a supported transport, build_config must
        // keep it as the selector default instead of falling back.
        let profile = fixture_profile();
        assert_eq!(profile.active_tag.as_deref(), Some(XHTTP_TAG));

        let split = SplitConfig::default();
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");

        let outbounds = config["outbounds"].as_array().expect("outbounds array");
        let selector = outbounds
            .iter()
            .find(|o| o["type"] == "selector" && o["tag"] == "proxy")
            .expect("selector outbound present");
        assert_eq!(selector["default"], XHTTP_TAG);

        assert!(outbounds
            .iter()
            .any(|o| o["type"] == "direct" && o["tag"] == "direct"));
        assert!(!outbounds.iter().any(|o| o["type"] == "block"));

        assert_eq!(
            config["experimental"]["clash_api"]["external_controller"],
            "127.0.0.1:9090"
        );
    }

    #[test]
    fn all_outbounds_unsupported_is_an_error() {
        // `splithttp` is the old Xray transport name; the bundled fork only
        // recognizes the current name, `xhttp`, so this is still genuinely
        // unsupported and should still produce an error.
        let outbounds = vec![json!({
            "type": "vless",
            "tag": "only-splithttp",
            "server": "203.0.113.10",
            "server_port": 443,
            "uuid": "11111111-2222-3333-4444-555555555555",
            "transport": { "type": "splithttp", "mode": "auto" }
        })];
        let profile = Profile::new("only unsupported", outbounds, &[]);
        let split = SplitConfig::default();
        let settings = BuildSettings::default();

        let err = build_config(&profile, &split, &settings).expect_err("should error");
        let message = err.to_string();
        assert!(message.contains("splithttp"), "message was: {message}");
        assert!(
            message.contains("No outbounds are supported"),
            "message was: {message}"
        );
    }

    #[test]
    fn splithttp_transport_is_also_excluded() {
        let outbounds = vec![
            json!({
                "type": "vless",
                "tag": "splithttp-out",
                "server": "203.0.113.10",
                "server_port": 443,
                "uuid": "11111111-2222-3333-4444-555555555555",
                "transport": { "type": "splithttp", "mode": "auto" }
            }),
            json!({
                "type": "vless",
                "tag": "ws-out",
                "server": "203.0.113.10",
                "server_port": 443,
                "uuid": "11111111-2222-3333-4444-555555555555",
                "transport": { "type": "ws", "path": "/" }
            }),
        ];
        let profile = Profile::new("mixed", outbounds, &[]);
        let split = SplitConfig::default();
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");

        let outbounds = config["outbounds"].as_array().expect("outbounds array");
        assert!(!outbounds.iter().any(|o| o["tag"] == "splithttp-out"));
        assert!(outbounds.iter().any(|o| o["tag"] == "ws-out"));
    }

    #[test]
    fn split_off_has_no_process_rules_and_final_proxy() {
        let profile = fixture_profile();
        let split = SplitConfig::default();
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");

        assert_eq!(config["route"]["final"], "proxy");
        let rules = config["route"]["rules"].as_array().expect("rules array");
        assert!(!rules.iter().any(|r| r.get("process_name").is_some()));
    }

    #[test]
    fn split_blacklist_routes_chrome_direct_and_final_proxy() {
        let profile = fixture_profile();
        let split = SplitConfig {
            mode: SplitMode::Blacklist,
            rules: vec![SplitRule::Process("chrome.exe".to_string())],
        };
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");

        assert_eq!(config["route"]["final"], "proxy");
        let rules = config["route"]["rules"].as_array().expect("rules array");
        let chrome_rule = rules
            .iter()
            .find(|r| r.get("process_name").is_some())
            .expect("process_name rule present");
        assert_eq!(chrome_rule["process_name"], json!(["chrome.exe"]));
        assert_eq!(chrome_rule["outbound"], "direct");
    }

    #[test]
    fn split_whitelist_routes_chrome_proxy_and_final_direct() {
        let profile = fixture_profile();
        let split = SplitConfig {
            mode: SplitMode::Whitelist,
            rules: vec![SplitRule::Process("chrome.exe".to_string())],
        };
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");

        assert_eq!(config["route"]["final"], "direct");
        let rules = config["route"]["rules"].as_array().expect("rules array");
        let chrome_rule = rules
            .iter()
            .find(|r| r.get("process_name").is_some())
            .expect("process_name rule present");
        assert_eq!(chrome_rule["process_name"], json!(["chrome.exe"]));
        assert_eq!(chrome_rule["outbound"], "proxy");
    }

    #[test]
    fn multiple_rules_of_same_field_are_grouped() {
        let profile = fixture_profile();
        let split = SplitConfig {
            mode: SplitMode::Blacklist,
            rules: vec![
                SplitRule::Process("chrome.exe".to_string()),
                SplitRule::Process("firefox.exe".to_string()),
                SplitRule::DomainSuffix("example.com".to_string()),
            ],
        };
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");
        let rules = config["route"]["rules"].as_array().expect("rules array");

        let process_rules: Vec<&Value> = rules
            .iter()
            .filter(|r| r.get("process_name").is_some())
            .collect();
        assert_eq!(process_rules.len(), 1);
        assert_eq!(
            process_rules[0]["process_name"],
            json!(["chrome.exe", "firefox.exe"])
        );

        let domain_rules: Vec<&Value> = rules
            .iter()
            .filter(|r| r.get("domain_suffix").is_some())
            .collect();
        assert_eq!(domain_rules.len(), 1);
    }

    #[test]
    fn blacklist_rules_are_grouped_before_hijack_dns_so_excluded_dns_is_direct() {
        // This is the fix for the real bug: an excluded (blacklisted) app's
        // DNS lookups must never be hijacked/proxied, only its blacklist
        // rule(s) may run before `hijack-dns`. If the ordering regresses,
        // an excluded game's DNS resolves via the proxy again even though
        // its traffic is direct, which can make it connect to a
        // wrong-region relay/matchmaking server.
        let profile = fixture_profile();
        let split = SplitConfig {
            mode: SplitMode::Blacklist,
            rules: vec![SplitRule::Process("dota2.exe".to_string())],
        };
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");
        let rules = config["route"]["rules"].as_array().expect("rules array");

        assert_eq!(rules[0]["action"], "sniff");

        let process_idx = rules
            .iter()
            .position(|r| r.get("process_name").is_some())
            .expect("process_name rule present");
        let hijack_idx = rules
            .iter()
            .position(|r| r["action"] == "hijack-dns")
            .expect("hijack-dns rule present");
        let private_idx = rules
            .iter()
            .position(|r| r.get("ip_is_private").is_some())
            .expect("ip_is_private rule present");

        assert!(
            process_idx < hijack_idx,
            "blacklisted process rule (index {process_idx}) must come before \
             hijack-dns (index {hijack_idx}) so its DNS goes direct too"
        );
        assert!(hijack_idx < private_idx);
        assert_eq!(rules[process_idx]["outbound"], "direct");
        assert_eq!(config["route"]["final"], "proxy");
    }

    #[test]
    fn whitelist_rules_come_after_hijack_dns_and_private_direct() {
        let profile = fixture_profile();
        let split = SplitConfig {
            mode: SplitMode::Whitelist,
            rules: vec![SplitRule::Process("chrome.exe".to_string())],
        };
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");
        let rules = config["route"]["rules"].as_array().expect("rules array");

        let hijack_idx = rules
            .iter()
            .position(|r| r["action"] == "hijack-dns")
            .expect("hijack-dns rule present");
        let private_idx = rules
            .iter()
            .position(|r| r.get("ip_is_private").is_some())
            .expect("ip_is_private rule present");
        let process_idx = rules
            .iter()
            .position(|r| r.get("process_name").is_some())
            .expect("process_name rule present");

        assert_eq!(rules[0]["action"], "sniff");
        assert!(hijack_idx < private_idx);
        assert!(private_idx < process_idx);
        assert_eq!(config["route"]["final"], "direct");
    }

    #[test]
    fn domain_regex_and_process_path_regex_rules_map_to_singbox_fields() {
        let profile = fixture_profile();
        let split = SplitConfig {
            mode: SplitMode::Blacklist,
            rules: vec![
                SplitRule::DomainRegex(r"^ads\.".to_string()),
                SplitRule::ProcessPathRegex(r"C:\\Games\\.*".to_string()),
            ],
        };
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");
        let rules = config["route"]["rules"].as_array().expect("rules array");

        let domain_regex_rule = rules
            .iter()
            .find(|r| r.get("domain_regex").is_some())
            .expect("domain_regex rule present");
        assert_eq!(domain_regex_rule["domain_regex"], json!([r"^ads\."]));
        assert_eq!(domain_regex_rule["outbound"], "direct");

        let process_path_regex_rule = rules
            .iter()
            .find(|r| r.get("process_path_regex").is_some())
            .expect("process_path_regex rule present");
        assert_eq!(
            process_path_regex_rule["process_path_regex"],
            json!([r"C:\\Games\\.*"])
        );
        assert_eq!(process_path_regex_rule["outbound"], "direct");
    }

    #[test]
    fn log_level_flows_into_log_block() {
        let profile = fixture_profile();
        let split = SplitConfig::default();
        let settings = BuildSettings {
            log_level: "trace".to_string(),
            ..BuildSettings::default()
        };
        let config = build_config(&profile, &split, &settings).expect("build should succeed");

        assert_eq!(config["log"]["level"], "trace");
        assert_eq!(config["log"]["timestamp"], true);
    }

    #[test]
    fn default_build_settings_log_level_is_info() {
        assert_eq!(BuildSettings::default().log_level, "info");
    }

    #[test]
    fn config_is_valid_json_and_round_trips() {
        let profile = fixture_profile();
        let split = SplitConfig::default();
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");

        let text = serde_json::to_string(&config).expect("serialize");
        let parsed: Value = serde_json::from_str(&text).expect("deserialize");
        assert_eq!(parsed, config);
    }

    #[test]
    fn always_has_private_ip_direct_and_dns_hijack_rules_first() {
        let profile = fixture_profile();
        let split = SplitConfig::default();
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");
        let rules = config["route"]["rules"].as_array().expect("rules array");

        assert_eq!(rules[0]["action"], "sniff");
        assert_eq!(rules[1]["action"], "hijack-dns");
        assert_eq!(rules[2]["ip_is_private"], true);
        assert_eq!(rules[2]["outbound"], "direct");
    }

    /// Real-binary gate: only runs when `SINGBOX_BIN` points at a sing-box
    /// executable, so CI (which has no binary available) stays green.
    ///
    /// Run locally with:
    /// ```text
    /// SINGBOX_BIN=/path/to/sing-box cargo test -p wisp-core
    /// ```
    #[test]
    fn generated_configs_pass_real_singbox_check() {
        let Ok(bin) = std::env::var("SINGBOX_BIN") else {
            eprintln!(
                "skipping generated_configs_pass_real_singbox_check: SINGBOX_BIN not set \
                 (set it to a sing-box binary path to run this gate)"
            );
            return;
        };

        let profile = fixture_profile();
        let settings = BuildSettings::default();
        let modes = [
            ("off", SplitConfig::default()),
            (
                "blacklist",
                SplitConfig {
                    mode: SplitMode::Blacklist,
                    rules: vec![
                        SplitRule::Process("dota2.exe".to_string()),
                        SplitRule::Process("steam.exe".to_string()),
                        SplitRule::IpCidr("10.0.0.0/8".to_string()),
                        SplitRule::DomainSuffix("steampowered.com".to_string()),
                        SplitRule::DomainRegex(r"^ads\..*\.example\.com$".to_string()),
                        SplitRule::ProcessPathRegex(
                            r"C:\\Program Files \(x86\)\\Steam\\.*".to_string(),
                        ),
                    ],
                },
            ),
            (
                "whitelist",
                SplitConfig {
                    mode: SplitMode::Whitelist,
                    rules: vec![
                        SplitRule::Process("chrome.exe".to_string()),
                        SplitRule::DomainSuffix("example.com".to_string()),
                    ],
                },
            ),
        ];

        let dir = std::env::temp_dir().join(format!(
            "wisp-singbox-check-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir for config check");

        for (name, split) in &modes {
            let config = build_config(&profile, split, &settings).expect("build should succeed");
            let path = dir.join(format!("{name}.json"));
            let mut file = std::fs::File::create(&path).expect("create config file");
            file.write_all(
                serde_json::to_string_pretty(&config)
                    .expect("serialize config")
                    .as_bytes(),
            )
            .expect("write config file");

            let output = Command::new(&bin)
                .arg("check")
                .arg("-c")
                .arg(&path)
                .output()
                .unwrap_or_else(|err| panic!("failed to run {bin} check: {err}"));

            assert!(
                output.status.success(),
                "sing-box check failed for {name} mode:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn camel_to_snake_converts_xray_field_names() {
        assert_eq!(camel_to_snake("xPaddingBytes"), "x_padding_bytes");
        assert_eq!(
            camel_to_snake("scMaxEachPostBytes"),
            "sc_max_each_post_bytes"
        );
        assert_eq!(
            camel_to_snake("scMaxBufferedPosts"),
            "sc_max_buffered_posts"
        );
        // Already-snake_case (and plain lowercase) keys pass through unchanged.
        assert_eq!(camel_to_snake("path"), "path");
        assert_eq!(camel_to_snake("x_padding_bytes"), "x_padding_bytes");
    }

    #[test]
    fn normalize_xhttp_transport_renames_camel_case_keys_and_preserves_values() {
        let mut transport = json!({
            "type": "xhttp",
            "mode": "auto",
            "path": "/",
            "xPaddingBytes": "50-100",
            "scMaxEachPostBytes": "1000000-1000000"
        });

        normalize_xhttp_transport(&mut transport);

        assert_eq!(transport["type"], "xhttp");
        assert_eq!(transport["mode"], "auto");
        assert_eq!(transport["path"], "/");
        assert_eq!(transport["x_padding_bytes"], "50-100");
        assert_eq!(transport["sc_max_each_post_bytes"], "1000000-1000000");
        assert!(transport.get("xPaddingBytes").is_none());
        assert!(transport.get("scMaxEachPostBytes").is_none());
    }

    #[test]
    fn normalize_xhttp_transport_injects_default_padding_when_missing() {
        let mut transport = json!({ "type": "xhttp", "mode": "auto", "path": "/" });
        normalize_xhttp_transport(&mut transport);
        assert_eq!(transport["x_padding_bytes"], "100-1000");
    }

    #[test]
    fn normalize_xhttp_transport_injects_default_padding_when_empty_or_zero() {
        let mut empty = json!({ "type": "xhttp", "xPaddingBytes": "" });
        normalize_xhttp_transport(&mut empty);
        assert_eq!(empty["x_padding_bytes"], "100-1000");

        let mut zero = json!({ "type": "xhttp", "xPaddingBytes": "0" });
        normalize_xhttp_transport(&mut zero);
        assert_eq!(zero["x_padding_bytes"], "100-1000");
    }

    #[test]
    fn normalize_xhttp_transport_leaves_already_snake_case_input_unchanged() {
        let mut transport = json!({
            "type": "xhttp",
            "mode": "auto",
            "x_padding_bytes": "200-500",
            "sc_max_each_post_bytes": "2000000-2000000"
        });
        let expected = transport.clone();

        normalize_xhttp_transport(&mut transport);

        assert_eq!(transport, expected);
    }
}
