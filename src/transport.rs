// @oagen-ignore-file
//! Pluggable HTTP transport.
//!
//! The SDK ships with a default `reqwest` impl behind the `reqwest` feature,
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
    pub method: Method,
    pub url: String,
    pub headers: HeaderMap,
    pub body: Option<Bytes>,
}

/// A response materialised into memory by the transport.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
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
#[derive(Debug)]
pub struct TransportError {
    pub kind: TransportErrorKind,
    pub source: Box<dyn StdError + Send + Sync>,
}

impl TransportError {
    pub fn new(
        kind: TransportErrorKind,
        source: impl Into<Box<dyn StdError + Send + Sync>>,
    ) -> Self {
        Self {
            kind,
            source: source.into(),
        }
    }

    pub fn connect(source: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        Self::new(TransportErrorKind::Connect, source)
    }

    pub fn timeout(source: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        Self::new(TransportErrorKind::Timeout, source)
    }

    pub fn other(source: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        Self::new(TransportErrorKind::Other, source)
    }

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
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, TransportError>;
}

/// Type-erased shared transport handle.
pub type SharedTransport = Arc<dyn HttpTransport>;
