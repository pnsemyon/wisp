//! wisp-core: pure, side-effect-free data model, share-link/JSON parsing,
//! and sing-box config generation for Wisp.
//!
//! This crate performs no I/O. Callers (wisp-engine, src-tauri, wisp-cli)
//! are responsible for reading input text, writing configs to disk, and
//! spawning the sing-box process.

pub mod error;
pub mod parse;
pub mod profile;
pub mod singbox;
pub mod split;

pub use error::{Result, WispError};
pub use parse::import;
pub use profile::Profile;
pub use singbox::{build_config, BuildSettings};
pub use split::{SplitConfig, SplitMode, SplitRule};
