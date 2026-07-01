//! Build a complete sing-box configuration from a `Profile` and
//! `SplitConfig`.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::error::Result;
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
}

impl Default for BuildSettings {
    fn default() -> Self {
        BuildSettings {
            mtu: 1280,
            clash_secret: String::new(),
            clash_port: 9090,
            socks_port: None,
        }
    }
}

/// Build a complete, ready-to-run sing-box config for `profile`, applying
/// `split` tunneling rules and `settings`.
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
    let tags = profile.tags();
    tracing::debug!(
        selector_tags = tags.len(),
        "build_config: selector outbound tags"
    );
    let default_tag = profile
        .active_tag
        .clone()
        .filter(|t| tags.contains(t))
        .or_else(|| tags.first().cloned());

    let mut outbounds: Vec<Value> = profile.outbounds.clone();
    outbounds.push(json!({
        "type": "selector",
        "tag": "proxy",
        "outbounds": tags,
        "default": default_tag,
    }));
    outbounds.push(json!({ "type": "direct", "tag": "direct" }));
    outbounds.push(json!({ "type": "block", "tag": "block" }));

    let mut inbounds = vec![json!({
        "type": "tun",
        "tag": "tun-in",
        "mtu": settings.mtu,
        "address": ["172.19.0.1/30"],
        "auto_route": true,
        "strict_route": true,
        "stack": "system",
        "endpoint_independent_nat": false,
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
        "log": { "level": "info", "timestamp": true },
        "dns": {
            "servers": [
                { "tag": "dns-remote", "address": "tls://8.8.8.8", "detour": "proxy" },
                { "tag": "dns-local", "address": "local", "detour": "direct" }
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
    let mut rules: Vec<Value> = vec![
        json!({ "action": "sniff" }),
        json!({ "protocol": "dns", "action": "hijack-dns" }),
        json!({ "ip_is_private": true, "outbound": "direct" }),
    ];

    let final_outbound = match split.mode {
        SplitMode::Off => "proxy",
        SplitMode::Exclude => {
            rules.extend(rule_group(&split.rules, "direct"));
            "proxy"
        }
        SplitMode::Include => {
            rules.extend(rule_group(&split.rules, "proxy"));
            "direct"
        }
    };

    json!({
        "auto_detect_interface": true,
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

    fn fixture_profile() -> Profile {
        crate::parse::import(REAL_CONFIG_FIXTURE).expect("fixture should import")
    }

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
        assert_eq!(tun_inbounds[0]["endpoint_independent_nat"], false);
    }

    #[test]
    fn selector_contains_all_three_tags_and_clash_api_present() {
        let profile = fixture_profile();
        let split = SplitConfig::default();
        let settings = BuildSettings::default();
        let config = build_config(&profile, &split, &settings).expect("build should succeed");

        let outbounds = config["outbounds"].as_array().expect("outbounds array");
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
        assert!(selector_tags.contains(&"Bulgaria, Sophia-7w1t0rtt5a § 0".to_string()));
        assert!(selector_tags.contains(&"Bulgaria, Sophia-7w1t0rtt5a § 1".to_string()));
        assert!(selector_tags.contains(&"Bulgaria, Sophia, hysteria-7w1t0rtt5a § 2".to_string()));
        assert_eq!(selector["default"], "Bulgaria, Sophia-7w1t0rtt5a § 0");

        assert!(outbounds
            .iter()
            .any(|o| o["type"] == "direct" && o["tag"] == "direct"));
        assert!(outbounds
            .iter()
            .any(|o| o["type"] == "block" && o["tag"] == "block"));

        assert_eq!(
            config["experimental"]["clash_api"]["external_controller"],
            "127.0.0.1:9090"
        );
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
    fn split_exclude_routes_chrome_direct_and_final_proxy() {
        let profile = fixture_profile();
        let split = SplitConfig {
            mode: SplitMode::Exclude,
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
    fn split_include_routes_chrome_proxy_and_final_direct() {
        let profile = fixture_profile();
        let split = SplitConfig {
            mode: SplitMode::Include,
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
            mode: SplitMode::Exclude,
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
}
