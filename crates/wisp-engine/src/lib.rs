//! wisp-engine: runs and controls the sing-box engine process, and exposes
//! a small [`Engine`] trait so callers (the Tauri backend, the CLI, and
//! later a mobile embedding) can control it uniformly regardless of how
//! it's actually implemented.

pub mod clash_api;
pub mod engine;
pub mod resources;
pub mod singbox_process;

pub use clash_api::{ClashApi, ConnectionsSnapshot};
pub use engine::{Engine, EngineState, EngineStatus, TrafficStats};
pub use resources::{locate_resources, Resources};
pub use singbox_process::SingBoxProcess;
