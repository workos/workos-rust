// @oagen-ignore-file
//! Pluggable HTTP transport.
//!
//! The SDK ships with a default `reqwest` impl behind the `rustls-tls` and
//! `native-tls` features,
//! but any [`HttpTransport`] can be supplied to [`crate::ClientBuilder::transport`]
//! — useful for WASM environments, custom retry/observability layers, or to
//! share a single connection pool with the rest of your application.

use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode};

#[cfg(any(feature = "native-tls", feature = "rustls-tls"))]
mod reqwest_impl;

#[cfg(any(feature = "native-tls", feature = "rustls-tls"))]
pub use reqwest_impl::ReqwestTransport;

/// A single outbound HTTP request.
///
/// Construction is fully done by the SDK — transport impls only consume it.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// HTTP method (`GET`, `POST`, etc.).
    pub method: Method,
    /// Fully-resolved request URL, including any query string.
    pub url: String,
    /// All headers to attach to the request, including auth and user-agent.
    pub headers: HeaderMap,
    /// Optional request body. Already JSON-encoded for endpoints with a body.
    pub body: Option<Bytes>,
}

/// A response materialised into memory by the transport.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// HTTP status code returned by the server.
    pub status: StatusCode,
    /// Response headers verbatim.
    pub headers: HeaderMap,
    /// Response body bytes. Empty for 204 / no-content responses.
    pub body: Bytes,
}

/// Error category — drives retry classification in the [`crate::Client`].
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TransportErrorKind {
    /// TCP/TLS connection could not be established. Retryable.
    Connect,
    /// The request timed out. Retryable.
    Timeout,
    /// Anything else (DNS, body decode, etc.). Not retried by default.
    Other,
}

/// Transport-level failure (everything that isn't an HTTP response).
///
/// Wraps the underlying error so callers can downcast via [`std::error::Error::source`]
/// when they need transport-specific details.
#[derive(Debug)]
pub struct TransportError {
    /// Coarse-grained category used by the SDK's retry logic.
    pub kind: TransportErrorKind,
    /// The underlying error from the transport implementation.
    pub source: Box<dyn StdError + Send + Sync>,
}

impl TransportError {
    /// Construct a [`TransportError`] from a kind + underlying error.
    pub fn new(
        kind: TransportErrorKind,
        source: impl Into<Box<dyn StdError + Send + Sync>>,
    ) -> Self {
        Self {
            kind,
            source: source.into(),
        }
    }

    /// Convenience constructor for [`TransportErrorKind::Connect`].
    pub fn connect(source: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        Self::new(TransportErrorKind::Connect, source)
    }

    /// Convenience constructor for [`TransportErrorKind::Timeout`].
    pub fn timeout(source: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        Self::new(TransportErrorKind::Timeout, source)
    }

    /// Convenience constructor for [`TransportErrorKind::Other`].
    pub fn other(source: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        Self::new(TransportErrorKind::Other, source)
    }

    /// `true` when the SDK should retry on this error (connect/timeout).
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.kind,
            TransportErrorKind::Connect | TransportErrorKind::Timeout
        )
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            TransportErrorKind::Connect => write!(f, "connect error: {}", self.source),
            TransportErrorKind::Timeout => write!(f, "timeout: {}", self.source),
            TransportErrorKind::Other => write!(f, "transport error: {}", self.source),
        }
    }
}

impl StdError for TransportError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&*self.source)
    }
}

/// Pluggable HTTP transport. Impls must be `Send + Sync` because the SDK
/// holds the transport in an `Arc` and shares it across tasks.
#[async_trait]
pub trait HttpTransport: Send + Sync {
    /// Send `req` and return the resulting [`HttpResponse`]. Network-level
    /// failures must surface as [`TransportError`] so the SDK can decide
    /// whether to retry; HTTP errors (4xx/5xx) should still return `Ok`
    /// — the SDK inspects the status code itself.
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, TransportError>;
}

/// Type-erased shared transport handle.
pub type SharedTransport = Arc<dyn HttpTransport>;
