// @oagen-ignore-file
//! JWKS helper (H13) — JWKS URL builder + fetch + cache.

use std::sync::Arc;
use std::time::{Duration, Instant};

use http::{HeaderMap, Method};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock, watch};

use crate::client::{Client, DEFAULT_BASE_URL};
use crate::error::Error;
use crate::transport::{HttpRequest, SharedTransport};

/// A single JSON Web Key (RFC 7517). Common metadata fields are typed; the
/// remainder of the key material (e.g. `n`/`e` for RSA, `x`/`y`/`crv` for EC,
/// `k` for symmetric keys) is preserved verbatim under `other` so callers can
/// feed it into their JWT library of choice.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Jwk {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kty: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alg: Option<String>,
    #[serde(default, rename = "use", skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
    #[serde(flatten)]
    pub other: serde_json::Map<String, serde_json::Value>,
}

/// A JWK Set (RFC 7517 §5).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct JwkSet {
    pub keys: Vec<Jwk>,
}

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

// Only callers overlapping an active fetch share its outcome. The sender is
// owned by the fetching caller, so cancellation also wakes all waiters.
type SharedFetch = watch::Receiver<Option<Result<Arc<JwkSet>, String>>>;

/// Caches the JWKS for a given client. Default TTL 10 minutes.
pub struct JwksHelper {
    transport: SharedTransport,
    url: String,
    ttl: Duration,
    cache: RwLock<Option<Cached>>,
    fetch_lock: Mutex<Option<SharedFetch>>,
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
            fetch_lock: Mutex::new(None),
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
        self.fetch_or_refresh(false, None).await
    }

    async fn fetch_or_refresh(
        &self,
        refresh: bool,
        previous: Option<&Arc<JwkSet>>,
    ) -> Result<Arc<JwkSet>, Error> {
        let mut guard = self.fetch_lock.lock().await;
        if let Some(active) = guard.as_ref()
            && active.has_changed().is_ok()
        {
            let mut completion = active.clone();
            drop(guard);
            loop {
                if let Some(result) = completion.borrow_and_update().clone() {
                    return result.map_err(Error::Jwt);
                }
                completion
                    .changed()
                    .await
                    .map_err(|_| Error::Jwt("JWKS fetch cancelled".into()))?;
            }
        }
        // A successful fetch may have replaced the caller's observed keys.
        if let Some(c) = self.cache.read().await.as_ref()
            && c.fetched_at.elapsed() < self.ttl
            && (!refresh || !previous.is_some_and(|set| Arc::ptr_eq(set, &c.set)))
        {
            return Ok(c.set.clone());
        }
        let (completion, receiver) = watch::channel(None);
        *guard = Some(receiver);
        drop(guard);

        let result = self.fetch_and_cache().await;
        completion.send_replace(Some(
            result.as_ref().map(Arc::clone).map_err(ToString::to_string),
        ));
        // Closing the channel makes the next non-overlapping caller eligible
        // to fetch again. Only existing waiters consume this attempt's failure.
        drop(completion);
        result
    }

    // Only the active fetch calls this. Keep the last known-good cache
    // available to readers until a replacement has been fetched and parsed.
    async fn fetch_and_cache(&self) -> Result<Arc<JwkSet>, Error> {
        let req = HttpRequest {
            method: Method::GET,
            url: self.url.clone(),
            headers: HeaderMap::new(),
            body: None,
        };
        let resp = self.transport.execute(req).await.map_err(Error::Network)?;
        if !resp.status.is_success() {
            return Err(Error::Api(Box::new(crate::error::ApiError::from_response(
                resp.status.as_u16(),
                &resp.headers,
                &resp.body,
            ))));
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

    /// Refresh the cache, preserving cached keys on failure. Concurrent calls
    /// share the in-flight result, including failures, rather than refetching.
    pub async fn refresh(&self) -> Result<Arc<JwkSet>, Error> {
        let previous = self.cache.read().await.as_ref().map(|c| c.set.clone());
        self.refresh_if_unchanged(previous.as_ref()).await
    }

    /// Refresh only if the caller's observed key set has not been replaced.
    pub(crate) async fn refresh_if_unchanged(
        &self,
        previous: Option<&Arc<JwkSet>>,
    ) -> Result<Arc<JwkSet>, Error> {
        self.fetch_or_refresh(true, previous).await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::transport::{HttpResponse, HttpTransport, TransportError};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    #[derive(Default)]
    struct YieldingTransport {
        requests: AtomicUsize,
        in_flight: AtomicUsize,
        fail: AtomicBool,
    }

    #[async_trait::async_trait]
    impl HttpTransport for YieldingTransport {
        async fn execute(&self, _: HttpRequest) -> Result<HttpResponse, TransportError> {
            self.requests.fetch_add(1, Ordering::SeqCst);
            // Yield with a request in flight to deterministically expose overlap.
            assert_eq!(self.in_flight.fetch_add(1, Ordering::SeqCst), 0);
            tokio::task::yield_now().await;
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            if self.fail.load(Ordering::SeqCst) {
                return Err(TransportError::other("JWKS unavailable"));
            }
            Ok(HttpResponse {
                status: http::StatusCode::OK,
                headers: HeaderMap::new(),
                body: r#"{"keys":[{"kid":"trusted"}]}"#.into(),
            })
        }
    }

    #[tokio::test]
    async fn concurrent_fetches_and_refreshes_share_successful_replacements() {
        let transport = Arc::new(YieldingTransport::default());
        let helper = JwksHelper::with_transport(transport.clone(), "", "client_test");
        let (a, b, c) = tokio::join!(helper.fetch(), helper.fetch(), helper.fetch());
        let original = a.unwrap();
        assert!(Arc::ptr_eq(&original, &b.unwrap()));
        assert!(Arc::ptr_eq(&original, &c.unwrap()));
        assert_eq!(transport.requests.load(Ordering::SeqCst), 1);

        let (a, b, c) = tokio::join!(helper.refresh(), helper.refresh(), helper.refresh());
        let replacement = a.unwrap();
        assert!(!Arc::ptr_eq(&original, &replacement));
        assert!(Arc::ptr_eq(&replacement, &b.unwrap()));
        assert!(Arc::ptr_eq(&replacement, &c.unwrap()));
        // A caller that observed the old set before this refresh also reuses it.
        assert!(Arc::ptr_eq(
            &replacement,
            &helper.refresh_if_unchanged(Some(&original)).await.unwrap()
        ));
        assert_eq!(transport.requests.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn failed_refreshes_are_shared_and_cached_reads_remain_available() {
        let transport = Arc::new(YieldingTransport::default());
        let helper = JwksHelper::with_transport(transport.clone(), "", "client_test");
        let original = helper.fetch().await.unwrap();
        transport.fail.store(true, Ordering::SeqCst);
        transport.requests.store(0, Ordering::SeqCst);
        let (a, b, c, cached) = tokio::join!(
            biased;
            helper.refresh(),
            helper.refresh(),
            helper.refresh(),
            async {
                // All refresh futures have been polled, with one still in flight.
                assert_eq!(transport.in_flight.load(Ordering::SeqCst), 1);
                helper.fetch().await.unwrap()
            }
        );
        assert!(matches!(a, Err(Error::Network(_))));
        assert!(matches!(b, Err(Error::Jwt(_))));
        assert!(matches!(c, Err(Error::Jwt(_))));
        assert!(Arc::ptr_eq(&original, &cached));
        assert!(Arc::ptr_eq(&original, &helper.fetch().await.unwrap()));
        assert_eq!(transport.requests.load(Ordering::SeqCst), 1);

        // A later caller retries immediately; failures have no cooldown.
        assert!(matches!(helper.refresh().await, Err(Error::Network(_))));
        assert_eq!(transport.requests.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn failed_initial_fetches_share_failure_but_later_call_retries() {
        let transport = Arc::new(YieldingTransport::default());
        transport.fail.store(true, Ordering::SeqCst);
        let helper = JwksHelper::with_transport(transport.clone(), "", "client_test");
        let (a, b, c) = tokio::join!(helper.fetch(), helper.fetch(), helper.fetch());
        assert!(a.is_err());
        assert!(b.is_err());
        assert!(c.is_err());
        assert_eq!(transport.requests.load(Ordering::SeqCst), 1);
        assert!(helper.cache.read().await.is_none());

        transport.fail.store(false, Ordering::SeqCst);
        assert!(helper.fetch().await.is_ok());
        assert_eq!(transport.requests.load(Ordering::SeqCst), 2);
    }

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
