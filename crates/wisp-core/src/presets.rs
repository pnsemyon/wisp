//! Built-in split-tunnel presets.
//!
//! [`valve_gaming_preset`] returns rules that route all Valve/Steam traffic
//! (Steam client, Dota 2, CS, and the Steam Datagram Relay game servers)
//! DIRECT, so latency-sensitive game UDP bypasses the VPN instead of being
//! measured through the tunnel exit. The IP ranges are Valve's announced
//! networks (ASN 32590); the domains cover Steam web/content endpoints.
//!
//! Source of the IP ranges: RIPEstat announced-prefixes for AS32590
//! (fetched 2026-07-02). Update via `stat.ripe.net` if Valve re-announces.

use crate::split::SplitRule;

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
    for cidr in VALVE_IPV4.iter().chain(VALVE_IPV6.iter()) {
        rules.push(SplitRule::IpCidr((*cidr).to_string()));
    }
    for d in VALVE_DOMAINS {
        rules.push(SplitRule::DomainSuffix((*d).to_string()));
    }
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_has_ipv4_ipv6_and_domains() {
        let rules = valve_gaming_preset();
        assert_eq!(
            rules.len(),
            VALVE_IPV4.len() + VALVE_IPV6.len() + VALVE_DOMAINS.len()
        );
        // the SDR relay range seen in real logs must be covered
        assert!(VALVE_IPV4.contains(&"155.133.230.0/24"));
        assert!(rules
            .iter()
            .any(|r| matches!(r, SplitRule::DomainSuffix(d) if d == "steampowered.com")));
    }
}
