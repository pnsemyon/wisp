//! Thin client for sing-box's Clash-compatible HTTP API
//! (`experimental.clash_api` in the generated config).

use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;

/// HTTP client bound to a single running sing-box instance's Clash API.
#[derive(Debug, Clone)]
pub struct ClashApi {
    base: String,
    secret: String,
    http: reqwest::Client,
}

/// `GET /version` response body.
#[derive(Debug, Deserialize)]
struct VersionResponse {
    version: String,
}

/// Parsed response of `GET /connections`: aggregate traffic totals plus the
/// list of individual active connections (kept opaque as JSON since we
/// currently only need the totals).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ConnectionsSnapshot {
    #[serde(rename = "downloadTotal")]
    pub download_total: u64,
    #[serde(rename = "uploadTotal")]
    pub upload_total: u64,
    #[serde(default)]
    pub connections: Vec<serde_json::Value>,
}

impl ClashApi {
    pub fn new(port: u16, secret: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        ClashApi {
            base: format!("http://127.0.0.1:{port}"),
            secret,
            http,
        }
    }

    /// Attach the `Authorization: Bearer <secret>` header, unless the
    /// secret is empty (sing-box only requires the header when a secret is
    /// configured).
    fn maybe_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if self.secret.is_empty() {
            builder
        } else {
            builder.bearer_auth(&self.secret)
        }
    }

    /// `GET /version`: used as a cheap health check to confirm sing-box has
    /// finished starting up.
    pub async fn version(&self) -> Result<String> {
        let url = format!("{}/version", self.base);
        let resp = self
            .maybe_auth(self.http.get(&url))
            .send()
            .await
            .context("sending /version request")?
            .error_for_status()
            .context("/version returned an error status")?;
        let body: VersionResponse = resp.json().await.context("parsing /version response")?;
        Ok(body.version)
    }

    /// `GET /connections`: current traffic totals and active connections.
    pub async fn connections(&self) -> Result<ConnectionsSnapshot> {
        let url = format!("{}/connections", self.base);
        let resp = self
            .maybe_auth(self.http.get(&url))
            .send()
            .await
            .context("sending /connections request")?
            .error_for_status()
            .context("/connections returned an error status")?;
        resp.json::<ConnectionsSnapshot>()
            .await
            .context("parsing /connections response")
    }

    /// `PUT /proxies/<selector>` with body `{"name": <name>}`: switch the
    /// active outbound of a `selector`-type outbound.
    pub async fn switch_selector(&self, selector: &str, name: &str) -> Result<()> {
        let url = format!("{}/proxies/{selector}", self.base);
        let body = serde_json::json!({ "name": name });
        self.maybe_auth(self.http.put(&url))
            .json(&body)
            .send()
            .await
            .context("sending switch_selector request")?
            .error_for_status()
            .context("switch_selector returned an error status")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_connections_snapshot() {
        let json = r#"{"downloadTotal":123,"uploadTotal":45,"connections":[]}"#;
        let snapshot: ConnectionsSnapshot = serde_json::from_str(json).expect("valid json");
        assert_eq!(snapshot.download_total, 123);
        assert_eq!(snapshot.upload_total, 45);
        assert!(snapshot.connections.is_empty());
    }

    #[test]
    fn bearer_header_set_when_secret_present() {
        let api = ClashApi::new(9090, "s3cr3t".to_string());
        let req = api
            .maybe_auth(api.http.get(format!("{}/version", api.base)))
            .build()
            .expect("request should build");
        assert_eq!(
            req.headers().get("authorization").expect("header present"),
            "Bearer s3cr3t"
        );
    }

    #[test]
    fn no_auth_header_when_secret_empty() {
        let api = ClashApi::new(9090, String::new());
        let req = api
            .maybe_auth(api.http.get(format!("{}/version", api.base)))
            .build()
            .expect("request should build");
        assert!(req.headers().get("authorization").is_none());
    }
}
