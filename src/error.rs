// @oagen-ignore-file
use thiserror::Error;

/// All errors produced by the SDK.
#[derive(Debug, Error)]
pub enum Error {
    /// The API responded with a non-2xx status.
    #[error("API error {status}: {message}")]
    Api {
        status: u16,
        code: Option<String>,
        message: String,
    },

    /// A transport-level error from the HTTP client.
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Failed to decode a JSON payload.
    #[error("decode error: {0}")]
    Decode(#[from] serde_json::Error),

    /// The caller supplied an invalid configuration or parameter.
    #[error("invalid request: {0}")]
    Builder(String),
}

impl Error {
    pub fn is_unauthorized(&self) -> bool {
        matches!(self, Error::Api { status: 401, .. })
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Error::Api { status: 404, .. })
    }

    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Error::Api { status: 429, .. })
    }

    pub fn is_server_error(&self) -> bool {
        matches!(self, Error::Api { status, .. } if (500..=599).contains(status))
    }
}
