//! Import profiles from raw sing-box outbound JSON or `vless://` /
//! `hysteria2://` share links.

use std::collections::HashMap;

use serde_json::{Map, Value};
use url::Url;

use crate::error::{Result, WispError};
use crate::profile::Profile;

const SUPPORTED_TYPES: [&str; 5] = ["vless", "hysteria2", "trojan", "shadowsocks", "vmess"];

/// Import a `Profile` from text that is either:
/// - a sing-box `{"outbounds": [...]}` object,
/// - a bare `[...]` array of outbounds, or
/// - one or more `vless://` / `hysteria2://` (or `hy2://`) share links,
///   one per line.
pub fn import(text: &str) -> Result<Profile> {
    let text = trim_bom(text).trim();
    if text.is_empty() {
        return Err(WispError::Parse("input is empty".to_string()));
    }
    let result = if text.starts_with('{') || text.starts_with('[') {
        tracing::debug!("import: detected JSON input shape");
        import_json(text)
    } else {
        tracing::debug!("import: detected share-link input shape");
        import_links(text)
    };
    match &result {
        Ok(profile) => tracing::debug!(
            outbounds = profile.outbounds.len(),
            "import: produced profile"
        ),
        Err(err) => tracing::debug!(%err, "import: failed"),
    }
    result
}

fn trim_bom(text: &str) -> &str {
    text.strip_prefix('\u{FEFF}').unwrap_or(text)
}

fn import_json(text: &str) -> Result<Profile> {
    let value: Value = serde_json::from_str(text)?;

    let raw_outbounds: Vec<Value> = match value {
        Value::Object(mut map) => match map.remove("outbounds") {
            Some(Value::Array(arr)) => arr,
            _ => {
                return Err(WispError::Parse(
                    "expected an \"outbounds\" array".to_string(),
                ))
            }
        },
        Value::Array(arr) => arr,
        _ => {
            return Err(WispError::Parse(
                "expected a JSON object or array".to_string(),
            ))
        }
    };

    let outbounds: Vec<Value> = raw_outbounds
        .into_iter()
        .filter(|o| {
            o.get("type")
                .and_then(Value::as_str)
                .map(|t| SUPPORTED_TYPES.contains(&t))
                .unwrap_or(false)
        })
        .collect();

    if outbounds.is_empty() {
        return Err(WispError::Parse(
            "no supported outbounds (vless/hysteria2/trojan/shadowsocks/vmess) found".to_string(),
        ));
    }

    let name = outbounds
        .first()
        .and_then(|o| o.get("tag"))
        .and_then(Value::as_str)
        .map(strip_counter_suffix)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Imported".to_string());

    Ok(Profile::new(name, outbounds, &[]))
}

/// Strip a trailing sing-box "§ N" counter suffix (added by some export
/// tools to disambiguate outbounds sharing a base name), e.g.
/// `"Bulgaria, Sophia § 0"` -> `"Bulgaria, Sophia"`.
fn strip_counter_suffix(tag: &str) -> String {
    match tag.find('§') {
        Some(idx) => tag[..idx]
            .trim_end()
            .trim_end_matches(',')
            .trim_end()
            .to_string(),
        None => tag.trim().to_string(),
    }
}

fn import_links(text: &str) -> Result<Profile> {
    let mut outbounds = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let outbound = if line.starts_with("vless://") {
            parse_vless_link(line)?
        } else if line.starts_with("hysteria2://") || line.starts_with("hy2://") {
            parse_hysteria2_link(line)?
        } else {
            let scheme = line.split("://").next().unwrap_or(line);
            return Err(WispError::UnsupportedProtocol(scheme.to_string()));
        };
        outbounds.push(outbound);
    }

    if outbounds.is_empty() {
        return Err(WispError::Parse("no share links found".to_string()));
    }

    let name = outbounds
        .first()
        .and_then(|o| o.get("tag"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Imported".to_string());

    Ok(Profile::new(name, outbounds, &[]))
}

fn query_map(url: &Url) -> HashMap<String, String> {
    url.query_pairs().into_owned().collect()
}

fn link_name(url: &Url) -> String {
    url.fragment()
        .map(percent_decode)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Imported".to_string())
}

fn required_host(url: &Url) -> Result<String> {
    url.host_str()
        .map(str::to_string)
        .ok_or_else(|| WispError::Parse("share link missing host".to_string()))
}

fn required_port(url: &Url) -> Result<u16> {
    url.port()
        .ok_or_else(|| WispError::Parse("share link missing port".to_string()))
}

/// Parse a `vless://<uuid>@<host>:<port>?<params>#<name>` share link into a
/// sing-box `vless` outbound.
fn parse_vless_link(link: &str) -> Result<Value> {
    let url = Url::parse(link)?;
    if url.scheme() != "vless" {
        return Err(WispError::UnsupportedProtocol(url.scheme().to_string()));
    }

    let uuid = url.username();
    if uuid.is_empty() {
        return Err(WispError::Parse("vless link missing uuid".to_string()));
    }

    let host = required_host(&url)?;
    let port = required_port(&url)?;
    let params = query_map(&url);
    let name = link_name(&url);

    let mut outbound = Map::new();
    outbound.insert("type".to_string(), Value::String("vless".to_string()));
    outbound.insert("tag".to_string(), Value::String(name));
    outbound.insert("server".to_string(), Value::String(host));
    outbound.insert("server_port".to_string(), Value::from(port));
    outbound.insert("uuid".to_string(), Value::String(uuid.to_string()));

    if let Some(flow) = params.get("flow").filter(|f| !f.is_empty()) {
        outbound.insert("flow".to_string(), Value::String(flow.clone()));
    }

    let security = params.get("security").map(String::as_str).unwrap_or("");
    if security == "reality" || security == "tls" {
        let mut tls = Map::new();
        tls.insert("enabled".to_string(), Value::Bool(true));
        if let Some(sni) = params.get("sni") {
            tls.insert("server_name".to_string(), Value::String(sni.clone()));
        }
        if let Some(fp) = params.get("fp") {
            let mut utls = Map::new();
            utls.insert("enabled".to_string(), Value::Bool(true));
            utls.insert("fingerprint".to_string(), Value::String(fp.clone()));
            tls.insert("utls".to_string(), Value::Object(utls));
        }
        if security == "reality" {
            let mut reality = Map::new();
            reality.insert("enabled".to_string(), Value::Bool(true));
            if let Some(pbk) = params.get("pbk") {
                reality.insert("public_key".to_string(), Value::String(pbk.clone()));
            }
            if let Some(sid) = params.get("sid") {
                reality.insert("short_id".to_string(), Value::String(sid.clone()));
            }
            tls.insert("reality".to_string(), Value::Object(reality));
        }
        outbound.insert("tls".to_string(), Value::Object(tls));
    }

    if params.get("type").map(String::as_str) == Some("xhttp") {
        let mut transport = Map::new();
        transport.insert("type".to_string(), Value::String("xhttp".to_string()));
        if let Some(path) = params.get("path") {
            transport.insert("path".to_string(), Value::String(path.clone()));
        }
        outbound.insert("transport".to_string(), Value::Object(transport));
    }

    Ok(Value::Object(outbound))
}

/// Parse a `hysteria2://<password>@<host>:<port>?<params>#<name>` (or
/// `hy2://`) share link into a sing-box `hysteria2` outbound.
fn parse_hysteria2_link(link: &str) -> Result<Value> {
    let url = Url::parse(link)?;
    let scheme = url.scheme();
    if scheme != "hysteria2" && scheme != "hy2" {
        return Err(WispError::UnsupportedProtocol(scheme.to_string()));
    }

    let password = url.username();
    if password.is_empty() {
        return Err(WispError::Parse(
            "hysteria2 link missing password".to_string(),
        ));
    }

    let host = required_host(&url)?;
    let port = required_port(&url)?;
    let params = query_map(&url);
    let name = link_name(&url);

    let mut outbound = Map::new();
    outbound.insert("type".to_string(), Value::String("hysteria2".to_string()));
    outbound.insert("tag".to_string(), Value::String(name));
    outbound.insert("server".to_string(), Value::String(host));
    outbound.insert("server_port".to_string(), Value::from(port));
    outbound.insert("password".to_string(), Value::String(password.to_string()));

    let obfs_password = params
        .get("obfs-password")
        .or_else(|| params.get("obfs_password"));
    if let Some(obfs_password) = obfs_password {
        let obfs_type = params
            .get("obfs")
            .map(String::as_str)
            .unwrap_or("salamander");
        let mut obfs = Map::new();
        obfs.insert("type".to_string(), Value::String(obfs_type.to_string()));
        obfs.insert("password".to_string(), Value::String(obfs_password.clone()));
        outbound.insert("obfs".to_string(), Value::Object(obfs));
    }

    let mut tls = Map::new();
    tls.insert("enabled".to_string(), Value::Bool(true));
    if let Some(sni) = params.get("sni") {
        tls.insert("server_name".to_string(), Value::String(sni.clone()));
    }
    if params.get("insecure").map(String::as_str) == Some("1") {
        tls.insert("insecure".to_string(), Value::Bool(true));
    }
    outbound.insert("tls".to_string(), Value::Object(tls));

    Ok(Value::Object(outbound))
}

/// Minimal percent-decoder for URL fragments (which `url::Url::fragment()`
/// returns raw/undecoded, unlike query pairs).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 3 <= bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The user's real sing-box outbound config, used as the primary test
/// fixture for both `parse.rs` and `singbox.rs`.
#[cfg(test)]
pub(crate) const REAL_CONFIG_FIXTURE: &str = r#"{ "outbounds": [ { "type": "vless", "tag": "Bulgaria, Sophia-7w1t0rtt5a § 0", "server": "203.0.113.10", "server_port": 38563, "uuid": "11111111-2222-3333-4444-555555555555", "tls": { "enabled": true, "server_name": "www.amazon.com", "utls": { "enabled": true, "fingerprint": "firefox" }, "reality": { "enabled": true, "public_key": "ExamplePublicKeyAAAAAAAAAAAAAAAAAAAAAAAAAAA", "short_id": "0123456789abcd" } }, "transport": { "type": "xhttp", "mode": "auto", "path": "/", "xPaddingBytes": "100-1000", "scMaxEachPostBytes": "1000000-1000000" }, "packet_encoding": "xudp" }, { "type": "vless", "tag": "Bulgaria, Sophia-7w1t0rtt5a § 1", "server": "203.0.113.10", "server_port": 37381, "uuid": "11111111-2222-3333-4444-555555555555", "flow": "xtls-rprx-vision", "tls": { "enabled": true, "server_name": "www.amd.com", "utls": { "enabled": true, "fingerprint": "safari" }, "reality": { "enabled": true, "public_key": "ExamplePublicKeyBBBBBBBBBBBBBBBBBBBBBBBBBBB", "short_id": "abcdef" } }, "packet_encoding": "xudp" }, { "type": "hysteria2", "tag": "Bulgaria, Sophia, hysteria-7w1t0rtt5a § 2", "server": "203.0.113.10", "server_port": 56085, "obfs": { "type": "salamander", "password": "example-obfs-pass-5678" }, "password": "example-password-1234", "tls": { "enabled": true, "disable_sni": true, "server_name": "203.0.113.10" } } ] }"#;

#[cfg(test)]
mod tests {
    use super::*;

    const REAL_CONFIG: &str = REAL_CONFIG_FIXTURE;

    #[test]
    fn imports_full_singbox_json() {
        let profile = import(REAL_CONFIG).expect("import should succeed");
        assert_eq!(profile.outbounds.len(), 3);
        assert_eq!(profile.name, "Bulgaria, Sophia-7w1t0rtt5a");
        assert_eq!(
            profile.tags(),
            vec![
                "Bulgaria, Sophia-7w1t0rtt5a § 0".to_string(),
                "Bulgaria, Sophia-7w1t0rtt5a § 1".to_string(),
                "Bulgaria, Sophia, hysteria-7w1t0rtt5a § 2".to_string(),
            ]
        );
        assert_eq!(
            profile.active_tag,
            Some("Bulgaria, Sophia-7w1t0rtt5a § 0".to_string())
        );
    }

    #[test]
    fn imports_bare_array() {
        let value: Value = serde_json::from_str(REAL_CONFIG).expect("valid json");
        let outbounds = value.get("outbounds").expect("outbounds").clone();
        let text = serde_json::to_string(&outbounds).expect("serialize");
        let profile = import(&text).expect("import should succeed");
        assert_eq!(profile.outbounds.len(), 3);
    }

    #[test]
    fn filters_unsupported_outbound_types() {
        let text = r#"{"outbounds":[
            {"type":"direct","tag":"direct"},
            {"type":"block","tag":"block"},
            {"type":"vless","tag":"keep","server":"1.2.3.4","server_port":443,"uuid":"u"}
        ]}"#;
        let profile = import(text).expect("import should succeed");
        assert_eq!(profile.outbounds.len(), 1);
        assert_eq!(profile.tags(), vec!["keep".to_string()]);
    }

    #[test]
    fn rejects_object_without_outbounds() {
        let err = import(r#"{"foo": 1}"#).unwrap_err();
        assert!(matches!(err, WispError::Parse(_)));
    }

    #[test]
    fn strips_bom_and_whitespace() {
        let text = format!("\u{FEFF}  {REAL_CONFIG}  \n");
        let profile = import(&text).expect("import should succeed");
        assert_eq!(profile.outbounds.len(), 3);
    }

    #[test]
    fn parses_vless_reality_link_equivalent_to_fixture_outbound() {
        // Built from outbound 0 of the real config fixture.
        let link = "vless://11111111-2222-3333-4444-555555555555@203.0.113.10:38563?\
            security=reality&pbk=ExamplePublicKeyAAAAAAAAAAAAAAAAAAAAAAAAAAA&\
            sid=0123456789abcd&sni=www.amazon.com&fp=firefox&type=xhttp&path=%2F\
            #Bulgaria%2C%20Sophia-7w1t0rtt5a";

        let profile = import(link).expect("import should succeed");
        assert_eq!(profile.outbounds.len(), 1);
        let outbound = &profile.outbounds[0];

        assert_eq!(outbound["type"], "vless");
        assert_eq!(outbound["server"], "203.0.113.10");
        assert_eq!(outbound["server_port"], 38563);
        assert_eq!(outbound["uuid"], "11111111-2222-3333-4444-555555555555");
        assert_eq!(outbound["tag"], "Bulgaria, Sophia-7w1t0rtt5a");
        assert_eq!(outbound["tls"]["enabled"], true);
        assert_eq!(outbound["tls"]["server_name"], "www.amazon.com");
        assert_eq!(outbound["tls"]["utls"]["fingerprint"], "firefox");
        assert_eq!(outbound["tls"]["reality"]["enabled"], true);
        assert_eq!(
            outbound["tls"]["reality"]["public_key"],
            "ExamplePublicKeyAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        );
        assert_eq!(outbound["tls"]["reality"]["short_id"], "0123456789abcd");
        assert_eq!(outbound["transport"]["type"], "xhttp");
        assert_eq!(outbound["transport"]["path"], "/");
    }

    #[test]
    fn parses_vless_vision_link_without_reality_type_field() {
        let link = "vless://11111111-2222-3333-4444-555555555555@203.0.113.10:37381?\
            security=reality&flow=xtls-rprx-vision&pbk=ExamplePublicKeyBBBBBBBBBBBBBBBBBBBBBBBBBBB&\
            sid=abcdef&sni=www.amd.com&fp=safari#server1";
        let profile = import(link).expect("import should succeed");
        let outbound = &profile.outbounds[0];
        assert_eq!(outbound["flow"], "xtls-rprx-vision");
        assert!(outbound.get("transport").is_none());
        assert_eq!(outbound["tls"]["reality"]["short_id"], "abcdef");
    }

    #[test]
    fn parses_hysteria2_link() {
        let link = "hysteria2://example-password-1234@203.0.113.10:56085?\
            obfs=salamander&obfs-password=example-obfs-pass-5678&sni=203.0.113.10&insecure=1#hy2-server";
        let profile = import(link).expect("import should succeed");
        let outbound = &profile.outbounds[0];
        assert_eq!(outbound["type"], "hysteria2");
        assert_eq!(outbound["password"], "example-password-1234");
        assert_eq!(outbound["server"], "203.0.113.10");
        assert_eq!(outbound["server_port"], 56085);
        assert_eq!(outbound["obfs"]["type"], "salamander");
        assert_eq!(outbound["obfs"]["password"], "example-obfs-pass-5678");
        assert_eq!(outbound["tls"]["server_name"], "203.0.113.10");
        assert_eq!(outbound["tls"]["insecure"], true);
        assert_eq!(outbound["tag"], "hy2-server");
    }

    #[test]
    fn hy2_scheme_alias_works() {
        let link = "hy2://pw@example.com:443#name";
        let profile = import(link).expect("import should succeed");
        assert_eq!(profile.outbounds[0]["type"], "hysteria2");
    }

    #[test]
    fn multiple_links_become_multiple_outbounds() {
        let text = "vless://u@host1:443?security=tls#one\nhysteria2://pw@host2:443#two\n";
        let profile = import(text).expect("import should succeed");
        assert_eq!(profile.outbounds.len(), 2);
        assert_eq!(profile.name, "one");
    }

    #[test]
    fn unsupported_scheme_errors() {
        let err = import("ss://foo@host:443#x").unwrap_err();
        assert!(matches!(err, WispError::UnsupportedProtocol(_)));
    }

    #[test]
    fn strip_counter_suffix_variants() {
        assert_eq!(
            strip_counter_suffix("Bulgaria, Sophia-7w1t0rtt5a § 0"),
            "Bulgaria, Sophia-7w1t0rtt5a"
        );
        assert_eq!(strip_counter_suffix("No suffix here"), "No suffix here");
    }
}
