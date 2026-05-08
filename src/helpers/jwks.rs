// @oagen-ignore-file
//! JWKS helper (H13) — JWKS URL builder + fetch + cache.

use std::sync::Arc;
use std::time::{Duration, Instant};

use http::{HeaderMap, Method};
use jsonwebtoken::jwk::JwkSet;
use tokio::sync::RwLock;

use crate::client::{Client, DEFAULT_BASE_URL};
use crate::error::Error;
use crate::transport::{HttpRequest, SharedTransport};

/// Builds the JWKS URL for `client_id` against `base_url` (defaulting when empty).
pub fn jwks_url(base_url: &str, client_id: &str) -> String {
    let base = if base_url.is_empty() {
        DEFAULT_BASE_URL
    } else {
        base_url
    };
    format!("{base}/sso/jwks/{client_id}")
}

#[derive(Clone)]
struct Cached {
    set: Arc<JwkSet>,
    fetched_at: Instant,
}

/// Caches the JWKS for a given client. Default TTL 10 minutes.
pub struct JwksHelper {
    transport: SharedTransport,
    url: String,
    ttl: Duration,
    cache: RwLock<Option<Cached>>,
}

impl JwksHelper {
    /// Build a JWKS helper that uses the supplied transport.
    pub fn with_transport(
        transport: SharedTransport,
        base_url: impl AsRef<str>,
        client_id: impl AsRef<str>,
    ) -> Self {
        Self {
            transport,
            url: jwks_url(base_url.as_ref(), client_id.as_ref()),
            ttl: Duration::from_secs(600),
            cache: RwLock::new(None),
        }
    }

    /// Construct a JWKS helper from a client — reuses the client's transport.
    pub fn from_client(client: &Client) -> Self {
        Self::with_transport(client.transport(), client.base_url(), client.client_id())
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Returns the JWKS URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Fetches (and caches) the JWKS. A subsequent call within `ttl` returns
    /// the cached set without making a network request.
    pub async fn fetch(&self) -> Result<Arc<JwkSet>, Error> {
        if let Some(c) = self.cache.read().await.as_ref()
            && c.fetched_at.elapsed() < self.ttl
        {
            return Ok(c.set.clone());
        }
        let req = HttpRequest {
            method: Method::GET,
            url: self.url.clone(),
            headers: HeaderMap::new(),
            body: None,
        };
        let resp = self.transport.execute(req).await.map_err(Error::Network)?;
        if !resp.status.is_success() {
            return Err(Error::Api {
                status: resp.status.as_u16(),
                code: None,
                message: String::from_utf8_lossy(&resp.body).into_owned(),
            });
        }
        let set: JwkSet = serde_json::from_slice(&resp.body).map_err(Error::from)?;
        let arc = Arc::new(set);
        let mut guard = self.cache.write().await;
        *guard = Some(Cached {
            set: arc.clone(),
            fetched_at: Instant::now(),
        });
        Ok(arc)
    }

    /// Force-refresh the cache.
    pub async fn refresh(&self) -> Result<Arc<JwkSet>, Error> {
        {
            let mut guard = self.cache.write().await;
            *guard = None;
        }
        self.fetch().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_default_base() {
        assert_eq!(
            jwks_url("", "client_123"),
            "https://api.workos.com/sso/jwks/client_123"
        );
    }

    #[test]
    fn url_explicit_base() {
        assert_eq!(
            jwks_url("https://api.example", "client_x"),
            "https://api.example/sso/jwks/client_x"
        );
    }
}
