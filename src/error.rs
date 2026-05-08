// @oagen-ignore-file
use std::time::Duration;

use bytes::Bytes;
use http::HeaderMap;
use thiserror::Error;

use crate::transport::TransportError;

/// Structured WorkOS API error response. Populated for every non-2xx response.
#[derive(Debug, Clone)]
pub struct ApiError {
    /// HTTP status code.
    pub status: u16,
    /// WorkOS error code (parsed from `code` or `error` JSON fields when present).
    pub code: Option<String>,
    /// Human-readable error message (parsed from `message` / `error_description`,
    /// otherwise the raw response body).
    pub message: String,
    /// `x-request-id` response header, if the server emitted one. Always
    /// include this when reporting bugs to WorkOS.
    pub request_id: Option<String>,
    /// `Retry-After` interval parsed from the response, when present. For
    /// `429` and `503` responses servers commonly populate this.
    pub retry_after: Option<Duration>,
    /// All response headers, for advanced debugging or custom error mapping.
    pub headers: HeaderMap,
    /// Raw response body bytes — preserved verbatim so callers can inspect
    /// non-standard error shapes.
    pub raw_body: Bytes,
}

impl ApiError {
    pub(crate) fn from_response(status: u16, headers: &HeaderMap, body: &Bytes) -> Self {
        let (code, message) = parse_body(body);
        let request_id = headers
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let retry_after = headers
            .get(http::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_retry_after);
        Self {
            status,
            code,
            message,
            request_id,
            retry_after,
            headers: headers.clone(),
            raw_body: body.clone(),
        }
    }
}

fn parse_body(body: &Bytes) -> (Option<String>, String) {
    if body.is_empty() {
        return (None, String::new());
    }
    let raw = String::from_utf8_lossy(body).into_owned();
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) else {
        return (None, raw);
    };
    let obj = match value {
        serde_json::Value::Object(o) => o,
        _ => return (None, raw),
    };
    let code = obj
        .get("code")
        .or_else(|| obj.get("error"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let message = obj
        .get("message")
        .or_else(|| obj.get("error_description"))
        .or_else(|| obj.get("error"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or(raw);
    (code, message)
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    let trimmed = value.trim();
    if let Ok(secs) = trimmed.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    // RFC 7231 also allows an HTTP-date here. We don't pull in chrono just
    // for that; callers needing exact-timestamp semantics can read
    // `headers` directly.
    None
}

/// All errors produced by the SDK.
#[derive(Debug, Error)]
pub enum Error {
    /// The API responded with a non-2xx status.
    #[error("API error {status}: {message}{request_id}",
        status = .0.status,
        message = .0.message,
        request_id = .0.request_id.as_deref().map(|r| format!(" (request_id={r})")).unwrap_or_default())]
    Api(Box<ApiError>),

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
    /// Returns the structured API error payload, when this is an API error.
    pub fn api(&self) -> Option<&ApiError> {
        match self {
            Error::Api(e) => Some(e),
            _ => None,
        }
    }

    pub fn status(&self) -> Option<u16> {
        self.api().map(|e| e.status)
    }

    pub fn code(&self) -> Option<&str> {
        self.api().and_then(|e| e.code.as_deref())
    }

    pub fn request_id(&self) -> Option<&str> {
        self.api().and_then(|e| e.request_id.as_deref())
    }

    pub fn retry_after(&self) -> Option<Duration> {
        self.api().and_then(|e| e.retry_after)
    }

    pub fn is_unauthorized(&self) -> bool {
        matches!(self.status(), Some(401))
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self.status(), Some(404))
    }

    pub fn is_rate_limited(&self) -> bool {
        matches!(self.status(), Some(429))
    }

    pub fn is_server_error(&self) -> bool {
        matches!(self.status(), Some(s) if (500..=599).contains(&s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_with_code_and_message() {
        let body = Bytes::from_static(br#"{"code":"user_not_found","message":"User not found"}"#);
        let (code, msg) = parse_body(&body);
        assert_eq!(code.as_deref(), Some("user_not_found"));
        assert_eq!(msg, "User not found");
    }

    #[test]
    fn parses_oauth_style_error() {
        let body = Bytes::from_static(
            br#"{"error":"invalid_grant","error_description":"refresh token expired"}"#,
        );
        let (code, msg) = parse_body(&body);
        assert_eq!(code.as_deref(), Some("invalid_grant"));
        assert_eq!(msg, "refresh token expired");
    }

    #[test]
    fn falls_back_to_raw_body_on_plain_text() {
        let body = Bytes::from_static(b"upstream timeout");
        let (code, msg) = parse_body(&body);
        assert!(code.is_none());
        assert_eq!(msg, "upstream timeout");
    }

    #[test]
    fn parses_retry_after_seconds() {
        assert_eq!(parse_retry_after("30"), Some(Duration::from_secs(30)));
        assert_eq!(parse_retry_after("not-a-number"), None);
    }

    #[test]
    fn extracts_request_id_header() {
        let mut h = HeaderMap::new();
        h.insert("x-request-id", "req_abc123".parse().unwrap());
        let body = Bytes::from_static(br#"{"message":"boom"}"#);
        let api = ApiError::from_response(500, &h, &body);
        assert_eq!(api.request_id.as_deref(), Some("req_abc123"));
        assert_eq!(api.message, "boom");
    }
}
