// @oagen-ignore-file
use thiserror::Error;

use crate::transport::TransportError;

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

    /// A transport-level error from the configured [`crate::transport::HttpTransport`].
    #[error("network error: {0}")]
    Network(#[from] TransportError),

    /// Failed to decode a JSON payload.
    #[error("decode error: {0}")]
    Decode(#[from] serde_json::Error),

    /// The caller supplied an invalid configuration or parameter.
    #[error("invalid request: {0}")]
    Builder(String),

    /// Webhook signature verification failed.
    #[error("webhook error: {0}")]
    Webhook(String),

    /// Sealed session encrypt/decrypt failed.
    #[error("session error: {0}")]
    Session(String),

    /// Vault local crypto failed.
    #[error("vault crypto error: {0}")]
    VaultCrypto(String),

    /// JWT/JWKS verification failed.
    #[error("jwt error: {0}")]
    Jwt(String),

    /// Crypto primitive (HMAC/AES/PKCE) failed.
    #[error("crypto error: {0}")]
    Crypto(String),
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
