//! The `Engine` trait abstraction plus its supporting status/stats types.
//!
//! This indirection lets the rest of the app (Tauri backend, CLI) control
//! "whatever is running the sing-box config" without caring whether that's
//! the bundled `.exe` (desktop) or an embedded engine (mobile, later).

use async_trait::async_trait;
use serde_json::Value;

/// Point-in-time traffic totals and instantaneous speeds, in bytes and
/// bytes/sec respectively.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TrafficStats {
    pub up_bytes: u64,
    pub down_bytes: u64,
    pub up_speed: u64,
    pub down_speed: u64,
}

/// Lifecycle state of the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineState {
    Stopped,
    Starting,
    Running,
    Errored,
}

/// Full status snapshot of the engine.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EngineStatus {
    pub state: EngineState,
    pub active_tag: Option<String>,
    pub since_unix: Option<u64>,
    pub last_error: Option<String>,
}

/// Something that can run a sing-box-shaped config: start/stop it, report
/// its status and traffic, stream its logs, and switch the active outbound.
#[async_trait]
pub trait Engine: Send + Sync {
    async fn start(&self, config: Value) -> anyhow::Result<()>;
    async fn stop(&self) -> anyhow::Result<()>;
    async fn status(&self) -> EngineStatus;
    async fn stats(&self) -> anyhow::Result<TrafficStats>;
    async fn logs(&self, max_lines: usize) -> Vec<String>;
    async fn switch(&self, tag: &str) -> anyhow::Result<()>;
}
