//! Split-tunnel model: which mode is active and which rules select the
//! apps/domains/IPs affected by it.

use serde::{Deserialize, Serialize};

/// Whether split tunneling is disabled, or which direction the listed
/// rules go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitMode {
    /// Everything goes through the proxy.
    Off,
    /// Listed rules go direct, everything else is proxied. Persisted
    /// `split.json` files from before the `Exclude` -> `Blacklist` rename
    /// still use `"mode":"exclude"`, so that's kept as a deserialize alias.
    #[serde(alias = "exclude")]
    Blacklist,
    /// Only listed rules are proxied, everything else goes direct.
    /// Persisted `split.json` files from before the `Include` ->
    /// `Whitelist` rename still use `"mode":"include"`, so that's kept as a
    /// deserialize alias.
    #[serde(alias = "include")]
    Whitelist,
}

/// A single split-tunnel selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SplitRule {
    Process(String),
    ProcessPath(String),
    DomainSuffix(String),
    /// Regular expression matched against the connection domain.
    DomainRegex(String),
    /// Regular expression matched against the process path.
    ProcessPathRegex(String),
    IpCidr(String),
}

impl SplitRule {
    /// The sing-box route-rule field name and value this rule maps to,
    /// e.g. `("process_name", "chrome.exe")`.
    pub fn field(&self) -> (&'static str, &str) {
        match self {
            SplitRule::Process(v) => ("process_name", v.as_str()),
            SplitRule::ProcessPath(v) => ("process_path", v.as_str()),
            SplitRule::DomainSuffix(v) => ("domain_suffix", v.as_str()),
            SplitRule::DomainRegex(v) => ("domain_regex", v.as_str()),
            SplitRule::ProcessPathRegex(v) => ("process_path_regex", v.as_str()),
            SplitRule::IpCidr(v) => ("ip_cidr", v.as_str()),
        }
    }
}

/// The full split-tunnel configuration for a profile/connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitConfig {
    pub mode: SplitMode,
    pub rules: Vec<SplitRule>,
}

impl Default for SplitConfig {
    fn default() -> Self {
        SplitConfig {
            mode: SplitMode::Off,
            rules: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_off_and_empty() {
        let cfg = SplitConfig::default();
        assert_eq!(cfg.mode, SplitMode::Off);
        assert!(cfg.rules.is_empty());
    }

    #[test]
    fn field_mapping() {
        assert_eq!(
            SplitRule::Process("chrome.exe".into()).field(),
            ("process_name", "chrome.exe")
        );
        assert_eq!(
            SplitRule::ProcessPath("C:\\a.exe".into()).field(),
            ("process_path", "C:\\a.exe")
        );
        assert_eq!(
            SplitRule::DomainSuffix("x.com".into()).field(),
            ("domain_suffix", "x.com")
        );
        assert_eq!(
            SplitRule::DomainRegex("^ads\\.".into()).field(),
            ("domain_regex", "^ads\\.")
        );
        assert_eq!(
            SplitRule::ProcessPathRegex("C:\\\\Games\\\\.*".into()).field(),
            ("process_path_regex", "C:\\\\Games\\\\.*")
        );
        assert_eq!(
            SplitRule::IpCidr("1.2.3.0/24".into()).field(),
            ("ip_cidr", "1.2.3.0/24")
        );
    }

    #[test]
    fn serde_roundtrip() {
        let mode = SplitMode::Blacklist;
        let json = serde_json::to_string(&mode).expect("serialize");
        assert_eq!(json, "\"blacklist\"");

        let mode = SplitMode::Whitelist;
        let json = serde_json::to_string(&mode).expect("serialize");
        assert_eq!(json, "\"whitelist\"");

        let rule = SplitRule::Process("chrome.exe".into());
        let json = serde_json::to_value(&rule).expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({"kind": "process", "value": "chrome.exe"})
        );
    }

    #[test]
    fn old_persisted_exclude_and_include_mode_names_still_deserialize() {
        // Existing `split.json` files on disk from before the rename use
        // the old mode names; they must keep loading as the renamed
        // variants rather than failing to deserialize.
        let cfg: SplitConfig =
            serde_json::from_str(r#"{"mode":"exclude","rules":[]}"#).expect("deserialize");
        assert_eq!(cfg.mode, SplitMode::Blacklist);

        let cfg: SplitConfig =
            serde_json::from_str(r#"{"mode":"include","rules":[]}"#).expect("deserialize");
        assert_eq!(cfg.mode, SplitMode::Whitelist);

        // New names also deserialize.
        let cfg: SplitConfig =
            serde_json::from_str(r#"{"mode":"blacklist","rules":[]}"#).expect("deserialize");
        assert_eq!(cfg.mode, SplitMode::Blacklist);

        let cfg: SplitConfig =
            serde_json::from_str(r#"{"mode":"whitelist","rules":[]}"#).expect("deserialize");
        assert_eq!(cfg.mode, SplitMode::Whitelist);
    }
}
