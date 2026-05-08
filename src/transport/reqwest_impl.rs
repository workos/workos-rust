// @oagen-ignore-file
//! Default `reqwest`-backed [`HttpTransport`] impl.
//!
//! Gated by the `reqwest` Cargo feature (default-on).

use std::time::Duration;

use async_trait::async_trait;

use super::{HttpRequest, HttpResponse, HttpTransport, TransportError};

/// Default transport. Wraps a [`reqwest::Client`].
#[derive(Clone)]
pub struct ReqwestTransport {
    inner: reqwest::Client,
}

impl ReqwestTransport {
    /// Build a transport from an existing `reqwest::Client` — useful when you
    /// want to share a connection pool with other code.
    pub fn from_client(client: reqwest::Client) -> Self {
        Self { inner: client }
    }

    /// Build a transport with the given request timeout and an empty default
    /// header map. Used by [`crate::ClientBuilder::build`] when no custom
    /// transport is supplied.
    pub fn with_timeout(timeout: Duration) -> Self {
        let inner = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client builder failed");
        Self { inner }
    }
}

impl Default for ReqwestTransport {
    fn default() -> Self {
        Self::from_client(reqwest::Client::new())
    }
}

#[async_trait]
impl HttpTransport for ReqwestTransport {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, TransportError> {
        let mut builder = self.inner.request(req.method, &req.url);
        for (name, value) in req.headers.iter() {
            builder = builder.header(name.clone(), value.clone());
        }
        if let Some(body) = req.body {
            builder = builder.body(body);
        }
        let resp = builder.send().await.map_err(map_err)?;
        let status = resp.status();
        let headers = resp.headers().clone();
        let body = resp.bytes().await.map_err(map_err)?;
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

fn map_err(e: reqwest::Error) -> TransportError {
    if e.is_connect() {
        TransportError::connect(e)
    } else if e.is_timeout() {
        TransportError::timeout(e)
    } else {
        TransportError::other(e)
    }
}
