//! Error types for wisp-core.

use thiserror::Error;

/// All errors that can occur while parsing profiles, share links, or
/// building sing-box configs. `wisp-core` performs no I/O, so this type
/// only covers data/format errors.
#[derive(Debug, Error)]
pub enum WispError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("unsupported protocol: {0}")]
    UnsupportedProtocol(String),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("url error: {0}")]
    Url(#[from] url::ParseError),

    #[error("base64 error: {0}")]
    Base64(String),

    #[error("{0}")]
    Other(String),
}

/// Convenience alias used throughout wisp-core.
pub type Result<T> = std::result::Result<T, WispError>;
