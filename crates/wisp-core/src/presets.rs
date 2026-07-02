//! Built-in split-tunnel presets.
//!
//! A preset is stored in a [`SplitConfig`](crate::split::SplitConfig) as a
//! single [`SplitRule::Preset`] entry (so the UI shows one row for a whole
//! category) and expanded into concrete rules by [`expand_rules`] just before
//! config generation. This is the extension point for future one-click
//! categories — add an id to [`preset_rules`] and a label to [`preset_label`].
//!
//! [`valve_gaming_preset`] returns rules that route all Valve/Steam traffic
//! (Steam client, Dota 2, CS, and the Steam Datagram Relay game servers)
//! DIRECT, so latency-sensitive game UDP bypasses the VPN instead of being
//! measured through the tunnel exit. It combines three complementary levers so
//! coverage doesn't depend on any single one: the game **processes** (catches
//! all their traffic regardless of destination), Valve's announced **IP
//! ranges** (ASN 32590 — catches the SDR relays even when process matching
//! misses short-lived UDP), and Steam **domains** (steers DNS + sniffed
//! connections direct).
//!
//! Source of the IP ranges: RIPEstat announced-prefixes for AS32590
//! (fetched 2026-07-02). Update via `stat.ripe.net` if Valve re-announces.

use crate::split::SplitRule;

/// Preset id for the Valve/Steam gaming bundle (the `value` of a
/// [`SplitRule::Preset`]).
pub const VALVE_PRESET_ID: &str = "valve";

/// Valve/Steam game & client executables. Excluding by process name catches
/// everything the game does — including the early Steam-networking handshake
/// and short-lived SDR UDP probes — regardless of which IP it lands on.
pub const VALVE_PROCESSES: &[&str] = &[
    "steam.exe",
    "steamwebhelper.exe",
    "steamservice.exe",
    "dota2.exe",
    "cs2.exe",
    "csgo.exe",
    "hl2.exe",
];

/// Valve/Steam IPv4 networks (AS32590).
pub const VALVE_IPV4: &[&str] = &[
    "103.10.124.0/24",
    "103.10.125.0/24",
    "103.28.54.0/24",
    "146.66.152.0/24",
    "146.66.155.0/24",
    "155.133.224.0/24",
    "155.133.225.0/24",
    "155.133.226.0/24",
    "155.133.227.0/24",
    "155.133.228.0/24",
    "155.133.229.0/24",
    "155.133.230.0/24",
    "155.133.236.0/23",
    "155.133.238.0/24",
    "155.133.239.0/24",
    "155.133.240.0/23",
    "155.133.244.0/24",
    "155.133.246.0/24",
    "155.133.248.0/24",
    "155.133.249.0/24",
    "155.133.250.0/24",
    "155.133.251.0/24",
    "155.133.252.0/24",
    "155.133.254.0/24",
    "155.133.255.0/24",
    "162.254.192.0/24",
    "162.254.193.0/24",
    "162.254.194.0/24",
    "162.254.195.0/24",
    "162.254.196.0/24",
    "162.254.197.0/24",
    "162.254.198.0/24",
    "162.254.199.0/24",
    "185.25.180.0/24",
    "185.25.182.0/24",
    "185.25.183.0/24",
    "192.69.96.0/22",
    "205.196.6.0/24",
    "208.64.200.0/24",
    "208.64.201.0/24",
    "208.64.202.0/24",
    "208.64.203.0/24",
    "208.78.164.0/22",
    "45.121.184.0/24",
];

/// Valve/Steam IPv6 networks (AS32590).
pub const VALVE_IPV6: &[&str] = &[
    "2404:3fc0:1::/48",
    "2404:3fc0:2::/48",
    "2404:3fc0:3::/48",
    "2404:3fc0:8::/48",
    "2404:3fc0:9::/48",
    "2404:3fc0::/48",
    "2404:3fc0:a::/48",
    "2602:801:f000::/48",
    "2602:801:f001::/48",
    "2602:801:f002::/48",
    "2602:801:f003::/48",
    "2602:801:f005::/48",
    "2602:801:f006::/48",
    "2602:801:f007::/48",
    "2602:801:f008::/48",
    "2602:801:f009::/48",
    "2602:801:f00a::/48",
    "2602:801:f00b::/48",
    "2602:801:f00d::/48",
    "2a01:bc80:1::/48",
    "2a01:bc80:2::/48",
    "2a01:bc80:3::/48",
    "2a01:bc80:4::/48",
    "2a01:bc80:5::/48",
    "2a01:bc80:6::/48",
    "2a01:bc80:7::/48",
    "2a01:bc80:8::/48",
    "2a01:bc80:9::/48",
    "2a01:bc80::/48",
    "2a01:bc80:a::/48",
    "2a01:bc80:b::/48",
    "2a01:bc80:c::/48",
];

/// Steam/Valve web + content domain suffixes.
pub const VALVE_DOMAINS: &[&str] = &[
    "steampowered.com",
    "steamcommunity.com",
    "steamstatic.com",
    "steamcontent.com",
    "steamusercontent.com",
    "steamserver.net",
    "steamgames.com",
    "valvesoftware.com",
    "dota2.com",
    "counter-strike.net",
    "steam-chat.com",
];

/// Rules that send all Valve/Steam/game traffic direct (add to a Blacklist).
pub fn valve_gaming_preset() -> Vec<SplitRule> {
    let mut rules = Vec::new();
    for p in VALVE_PROCESSES {
        rules.push(SplitRule::Process((*p).to_string()));
    }
    for cidr in VALVE_IPV4.iter().chain(VALVE_IPV6.iter()) {
        rules.push(SplitRule::IpCidr((*cidr).to_string()));
    }
    for d in VALVE_DOMAINS {
        rules.push(SplitRule::DomainSuffix((*d).to_string()));
    }
    rules
}

/// The concrete rules a preset id expands to, or `None` for an unknown id.
pub fn preset_rules(id: &str) -> Option<Vec<SplitRule>> {
    match id {
        VALVE_PRESET_ID => Some(valve_gaming_preset()),
        _ => None,
    }
}

/// Human-readable label for a preset id, for the UI. `None` for unknown ids.
pub fn preset_label(id: &str) -> Option<&'static str> {
    match id {
        VALVE_PRESET_ID => Some("Valve / Steam games (Dota 2, CS, Steam)"),
        _ => None,
    }
}

/// Expand every [`SplitRule::Preset`] into its concrete rules, leaving all
/// other rules untouched and in place. An unknown preset id is dropped with a
/// warning (a forward/rolled-back config referencing a preset this build
/// doesn't know shouldn't abort the whole connection). The result contains no
/// `Preset` entries, so it's safe to feed to routing/DNS generation.
pub fn expand_rules(rules: &[SplitRule]) -> Vec<SplitRule> {
    let mut out = Vec::with_capacity(rules.len());
    for rule in rules {
        match rule {
            SplitRule::Preset(id) => match preset_rules(id) {
                Some(expanded) => out.extend(expanded),
                None => tracing::warn!(preset = %id, "expand_rules: unknown preset id, dropping"),
            },
            other => out.push(other.clone()),
        }
    }
    out
}

/// Whether `rule` is one this build considers part of the given preset id —
/// used to migrate/dedup older configs that stored a preset's rules expanded
/// (as individual entries) back into a single [`SplitRule::Preset`].
pub fn is_preset_member(id: &str, rule: &SplitRule) -> bool {
    match rule {
        SplitRule::Preset(existing) => existing == id,
        other => preset_rules(id).is_some_and(|rules| rules.contains(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_has_processes_ipv4_ipv6_and_domains() {
        let rules = valve_gaming_preset();
        assert_eq!(
            rules.len(),
            VALVE_PROCESSES.len() + VALVE_IPV4.len() + VALVE_IPV6.len() + VALVE_DOMAINS.len()
        );
        // the SDR relay range seen in real logs must be covered
        assert!(VALVE_IPV4.contains(&"155.133.230.0/24"));
        assert!(rules
            .iter()
            .any(|r| matches!(r, SplitRule::DomainSuffix(d) if d == "steampowered.com")));
        // process coverage catches game traffic regardless of destination
        assert!(rules
            .iter()
            .any(|r| matches!(r, SplitRule::Process(p) if p == "dota2.exe")));
    }

    #[test]
    fn expand_replaces_preset_and_leaves_others() {
        let input = vec![
            SplitRule::DomainSuffix("example.com".into()),
            SplitRule::Preset(VALVE_PRESET_ID.into()),
        ];
        let out = expand_rules(&input);
        // the plain rule survives; the preset entry itself is gone
        assert!(out.contains(&SplitRule::DomainSuffix("example.com".into())));
        assert!(!out.iter().any(|r| matches!(r, SplitRule::Preset(_))));
        // and it was replaced by the valve rules
        assert!(out.contains(&SplitRule::Process("dota2.exe".into())));
        assert_eq!(out.len(), 1 + valve_gaming_preset().len());
    }

    #[test]
    fn unknown_preset_is_dropped() {
        let out = expand_rules(&[SplitRule::Preset("nope".into())]);
        assert!(out.is_empty());
    }

    #[test]
    fn is_preset_member_matches_constituents_and_the_preset_itself() {
        assert!(is_preset_member(
            VALVE_PRESET_ID,
            &SplitRule::Process("dota2.exe".into())
        ));
        assert!(is_preset_member(
            VALVE_PRESET_ID,
            &SplitRule::IpCidr("155.133.230.0/24".into())
        ));
        assert!(is_preset_member(
            VALVE_PRESET_ID,
            &SplitRule::Preset(VALVE_PRESET_ID.into())
        ));
        assert!(!is_preset_member(
            VALVE_PRESET_ID,
            &SplitRule::DomainSuffix("example.com".into())
        ));
    }
}
