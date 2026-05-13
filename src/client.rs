// @oagen-ignore-file
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http::{
    HeaderMap, HeaderName, HeaderValue, Method,
    header::{AUTHORIZATION, RETRY_AFTER, USER_AGENT},
};
use rand::Rng;
use serde::{Serialize, de::DeserializeOwned};

use crate::error::{ApiError, Error};
use crate::transport::{HttpRequest, SharedTransport};

/// Per-request retry/idempotency policy. Override via [`RequestOptions::strategy`].
///
/// Stripe-style: callers can opt into stronger guarantees (or weaker — `Once`)
/// without reaching for the client-level `max_retries` knob. When unset, the
/// SDK applies its default policy: HTTP-safe methods (GET/HEAD/OPTIONS) and
/// requests carrying an `Idempotency-Key` header are retried on 429/5xx with
/// exponential backoff + jitter; everything else is sent once.
#[derive(Debug, Clone)]
pub enum RequestStrategy {
    /// Send the request exactly once. Never auto-retry.
    Once,
    /// Send with the given idempotency key. Auto-retry up to the client's
    /// configured `max_retries`. WorkOS uses the key to deduplicate retried
    /// mutations server-side.
    Idempotent(String),
    /// Retry up to `max_attempts` total times on 429/5xx with no inter-attempt
    /// sleep. Only safe for idempotent operations.
    Retry { max_attempts: u32 },
    /// Retry up to `max_attempts` total times on 429/5xx using exponential
    /// backoff. When `jitter` is true (recommended), each sleep is scaled by
    /// a uniform random factor in `[0.5, 1.5)` to avoid thundering herds.
    ExponentialBackoff { max_attempts: u32, jitter: bool },
}

impl RequestStrategy {
    fn max_attempts(&self, client_default: u32) -> u32 {
        match self {
            Self::Once => 1,
            Self::Idempotent(_) => client_default.saturating_add(1),
            Self::Retry { max_attempts } | Self::ExponentialBackoff { max_attempts, .. } => {
                *max_attempts
            }
        }
    }

    fn idempotency_key(&self) -> Option<&str> {
        match self {
            Self::Idempotent(k) => Some(k.as_str()),
            _ => None,
        }
    }

    fn jitter(&self) -> bool {
        matches!(self, Self::ExponentialBackoff { jitter: true, .. })
            || matches!(self, Self::Idempotent(_))
    }

    fn use_backoff(&self) -> bool {
        !matches!(self, Self::Retry { .. })
    }
}

/// Per-request overrides supplied to the `*_with_options` API methods.
///
/// All fields are optional; an empty `RequestOptions` is equivalent to the
/// default behaviour. Construct via [`RequestOptions::new`] or
/// [`Default::default`] and chain the builder-style setters.
#[derive(Debug, Default, Clone)]
pub struct RequestOptions {
    /// Sent as the `Idempotency-Key` HTTP header. Stripe-style: pass the same
    /// key on a retry to make the request safe to repeat.
    pub idempotency_key: Option<String>,
    /// Additional headers merged on top of the client's default headers.
    /// Later entries override the same name from this list.
    pub extra_headers: Vec<(HeaderName, HeaderValue)>,
    /// Optional per-request retry policy. Overrides the client-level default
    /// when set; when unset the SDK applies safety-aware retries.
    pub strategy: Option<RequestStrategy>,
}

impl RequestOptions {
    /// Construct an empty [`RequestOptions`]. Equivalent to [`Default::default`].
    ///
    /// ```
    /// use workos::RequestOptions;
    /// let opts = RequestOptions::new().idempotency_key("ik_42");
    /// # let _ = opts;
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the `Idempotency-Key` header for this request. Pass the same key
    /// when retrying a mutating request to make it safe to repeat — WorkOS
    /// recognises the key on the server side and returns the cached response
    /// for previously-seen calls instead of re-executing the side effect.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Append an arbitrary header to this request. Later entries with the
    /// same name override earlier ones; entries here override the client's
    /// default headers when names collide.
    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.extra_headers.push((name, value));
        self
    }

    /// Set a per-request [`RequestStrategy`]. Overrides the client-level
    /// retry policy for this single call. When the strategy is
    /// [`RequestStrategy::Idempotent`], the wrapped key is also sent as the
    /// `Idempotency-Key` header (overriding [`Self::idempotency_key`]).
    pub fn strategy(mut self, strategy: RequestStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }
}

/// Percent-encode a single URL path segment per RFC 3986. Used by generated
/// resource code to safely interpolate dynamic path parameters into request
/// URLs without letting reserved characters (`/`, `?`, `#`, spaces, etc.)
/// escape the segment they belong to.
#[doc(hidden)]
pub fn path_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Default base URL. Override via [`ClientBuilder::base_url`].
pub const DEFAULT_BASE_URL: &str = "https://api.workos.com";

/// Default request timeout (used by the built-in `reqwest` transport).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Cheap-to-clone WorkOS API client.
///
/// All state (transport, base URL, default headers, retry budget) lives behind
/// an `Arc`, so cloning hands out a new handle to the same underlying client.
/// Clone freely across tasks and store one client per process.
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

pub(crate) struct ClientInner {
    pub(crate) transport: SharedTransport,
    pub(crate) base_url: String,
    pub(crate) max_retries: u32,
    pub(crate) api_key: String,
    pub(crate) client_id: String,
    pub(crate) default_headers: HeaderMap,
}

/// Builder for [`Client`]. Construct via [`Client::builder`] and chain the
/// setter methods; finalize with [`Self::build`] (panics on invalid headers)
/// or [`Self::try_build`] (returns [`Error::Builder`] instead).
#[derive(Default)]
pub struct ClientBuilder {
    api_key: Option<String>,
    client_id: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    max_retries: Option<u32>,
    user_agent: Option<String>,
    transport: Option<SharedTransport>,
}

impl Client {
    /// Construct a new client with the given API key and default settings.
    ///
    /// Requires either the `native-tls` or `rustls-tls` feature (the latter is
    /// default-on). For custom transports use [`Client::builder`] with
    /// [`ClientBuilder::transport`].
    #[cfg(any(feature = "native-tls", feature = "rustls-tls"))]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::builder().api_key(api_key).build()
    }

    /// Begin building a client with custom configuration.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    pub(crate) async fn request_with_query<P: Serialize, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        params: &P,
    ) -> Result<R, Error> {
        self.request_with_query_opts(method, path, params, None)
            .await
    }

    pub(crate) async fn request_with_query_opts<P: Serialize, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        params: &P,
        opts: Option<&RequestOptions>,
    ) -> Result<R, Error> {
        let req = self.build_request(method.clone(), path, Some(params), None::<&()>, opts)?;
        self.send(req, method, opts).await
    }

    pub(crate) async fn request_with_body_opts<P: Serialize, B: Serialize, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        params: &P,
        body: Option<&B>,
        opts: Option<&RequestOptions>,
    ) -> Result<R, Error> {
        let req = self.build_request(method.clone(), path, Some(params), body, opts)?;
        self.send(req, method, opts).await
    }

    /// POST/PUT a JSON body and deserialize the response.
    pub(crate) async fn request_json<B: Serialize, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: &B,
    ) -> Result<R, Error> {
        let req = self.build_request(method.clone(), path, None::<&()>, Some(body), None)?;
        self.send(req, method, None).await
    }

    /// POST/PUT/DELETE/GET that does not deserialize a response body.
    pub(crate) async fn request_empty<B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<(), Error> {
        let req = self.build_request(method.clone(), path, None::<&()>, body, None)?;
        self.send_no_body(req, method, None).await
    }

    /// Variant of [`Self::request_with_query_opts`] that discards the response
    /// body. Used for endpoints whose OpenAPI schema declares no response
    /// content (DELETE, etc.).
    pub(crate) async fn request_with_query_opts_empty<P: Serialize>(
        &self,
        method: Method,
        path: &str,
        params: &P,
        opts: Option<&RequestOptions>,
    ) -> Result<(), Error> {
        let req = self.build_request(method.clone(), path, Some(params), None::<&()>, opts)?;
        self.send_no_body(req, method, opts).await
    }

    /// Variant of [`Self::request_with_body_opts`] that discards the response
    /// body.
    pub(crate) async fn request_with_body_opts_empty<P: Serialize, B: Serialize>(
        &self,
        method: Method,
        path: &str,
        params: &P,
        body: Option<&B>,
        opts: Option<&RequestOptions>,
    ) -> Result<(), Error> {
        let req = self.build_request(method.clone(), path, Some(params), body, opts)?;
        self.send_no_body(req, method, opts).await
    }

    /// Public base URL — needed by URL-builder helpers (AuthKit, SSO, JWKS).
    pub fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// Configured `client_id` (empty if not set). Used by helpers.
    pub fn client_id(&self) -> &str {
        &self.inner.client_id
    }

    /// Configured API key (empty if not set). Used by helpers that send it as
    /// `client_secret` in OAuth flows.
    pub fn api_key(&self) -> &str {
        &self.inner.api_key
    }

    /// Shared transport handle — exposed for helper modules that issue
    /// requests outside the standard API base URL (e.g. JWKS fetch).
    pub fn transport(&self) -> SharedTransport {
        self.inner.transport.clone()
    }

    /// Default headers (Authorization, User-Agent) — helpers building requests
    /// against the API base URL should attach these.
    pub fn default_headers(&self) -> &HeaderMap {
        &self.inner.default_headers
    }

    /// Passwordless (magic-link) helper.
    pub fn passwordless(&self) -> crate::helpers::PasswordlessApi<'_> {
        crate::helpers::PasswordlessApi { client: self }
    }

    /// Vault — KV operations + local AES-GCM crypto.
    pub fn vault(&self) -> crate::helpers::VaultApi<'_> {
        crate::helpers::VaultApi { client: self }
    }

    /// AuthKit helpers (URL builder, PKCE flows, device flow).
    pub fn authkit(&self) -> crate::helpers::AuthKitHelper<'_> {
        crate::helpers::AuthKitHelper { client: self }
    }

    /// SSO helpers (URL builder, PKCE flows, logout flow).
    pub fn sso_helpers(&self) -> crate::helpers::SsoHelper<'_> {
        crate::helpers::SsoHelper { client: self }
    }

    /// JWKS helper bound to this client's `client_id`.
    pub fn jwks(&self) -> crate::helpers::JwksHelper {
        crate::helpers::JwksHelper::from_client(self)
    }

    /// Construct a [`crate::helpers::SessionManager`] for an existing sealed
    /// session cookie.
    pub fn session<'a>(
        &'a self,
        sealed: impl Into<String>,
        password: impl Into<String>,
    ) -> crate::helpers::SessionManager<'a> {
        crate::helpers::SessionManager::new(Some(self), sealed, password)
    }

    fn build_request<P: Serialize, B: Serialize>(
        &self,
        method: Method,
        path: &str,
        query: Option<&P>,
        body: Option<&B>,
        opts: Option<&RequestOptions>,
    ) -> Result<HttpRequest, Error> {
        let mut url = format!("{}{}", self.inner.base_url, path);
        if let Some(p) = query {
            let qs = crate::query::encode_query(p)?;
            if !qs.is_empty() {
                let sep = if url.contains('?') { '&' } else { '?' };
                url.push(sep);
                url.push_str(&qs);
            }
        }

        let mut headers = self.inner.default_headers.clone();
        let body_bytes = if let Some(b) = body {
            let bytes = serde_json::to_vec(b).map_err(Error::from)?;
            headers.insert(
                http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            Some(Bytes::from(bytes))
        } else {
            None
        };

        if let Some(o) = opts {
            // RequestStrategy::Idempotent overrides any explicit idempotency_key.
            let strategy_key = o.strategy.as_ref().and_then(|s| s.idempotency_key());
            let key = strategy_key.or(o.idempotency_key.as_deref());
            if let Some(key) = key {
                let v = HeaderValue::from_str(key)
                    .map_err(|e| Error::Builder(format!("invalid idempotency key: {e}")))?;
                headers.insert(HeaderName::from_static("idempotency-key"), v);
            }
            for (name, value) in &o.extra_headers {
                headers.insert(name.clone(), value.clone());
            }
        }

        Ok(HttpRequest {
            method,
            url,
            headers,
            body: body_bytes,
        })
    }

    async fn send_no_body(
        &self,
        req: HttpRequest,
        method: Method,
        opts: Option<&RequestOptions>,
    ) -> Result<(), Error> {
        let resp = self.execute_with_retry(req, &method, opts).await?;
        let status = resp.status.as_u16();
        if (200..300).contains(&status) {
            Ok(())
        } else {
            Err(Error::Api(Box::new(ApiError::from_response(
                status,
                &resp.headers,
                &resp.body,
            ))))
        }
    }

    async fn send<R: DeserializeOwned>(
        &self,
        req: HttpRequest,
        method: Method,
        opts: Option<&RequestOptions>,
    ) -> Result<R, Error> {
        let resp = self.execute_with_retry(req, &method, opts).await?;
        let status = resp.status.as_u16();
        if (200..300).contains(&status) {
            serde_json::from_slice::<R>(&resp.body).map_err(Error::from)
        } else {
            Err(Error::Api(Box::new(ApiError::from_response(
                status,
                &resp.headers,
                &resp.body,
            ))))
        }
    }

    async fn execute_with_retry(
        &self,
        req: HttpRequest,
        method: &Method,
        opts: Option<&RequestOptions>,
    ) -> Result<crate::transport::HttpResponse, Error> {
        // Resolve the effective retry policy for this request:
        //  - per-request strategy (`opts.strategy`) wins,
        //  - otherwise fall back to the client-level `max_retries` budget,
        //    only if the call is safe to repeat (idempotent method or carries
        //    an Idempotency-Key header).
        let strategy = opts.and_then(|o| o.strategy.clone());
        let has_idempotency_key = opts
            .map(|o| {
                o.strategy
                    .as_ref()
                    .is_some_and(|s| s.idempotency_key().is_some())
                    || o.idempotency_key.is_some()
            })
            .unwrap_or(false);
        let safe_method = is_safe_method(method);
        let auto_retry_allowed = safe_method || has_idempotency_key;
        let max_attempts = match &strategy {
            Some(s) => s.max_attempts(self.inner.max_retries),
            None => {
                if auto_retry_allowed {
                    self.inner.max_retries.saturating_add(1)
                } else {
                    1
                }
            }
        };
        let use_backoff = strategy.as_ref().is_none_or(|s| s.use_backoff());
        let jitter = strategy.as_ref().map(|s| s.jitter()).unwrap_or(true);

        let mut attempt: u32 = 0;
        loop {
            let cloned = req.clone();
            let result = self.inner.transport.execute(cloned).await;
            match result {
                Ok(resp) => {
                    let status = resp.status.as_u16();
                    let retryable = status == 429 || (500..=599).contains(&status);
                    let attempts_used = attempt + 1;
                    if retryable && attempts_used < max_attempts {
                        let delay = match retry_after(&resp.headers) {
                            Some(d) => d,
                            None if use_backoff => backoff_delay(attempt + 1, jitter),
                            None => Duration::from_millis(0),
                        };
                        attempt += 1;
                        if !delay.is_zero() {
                            tokio::time::sleep(delay).await;
                        }
                        continue;
                    }
                    return Ok(resp);
                }
                Err(e) => {
                    let attempts_used = attempt + 1;
                    if e.is_retryable() && attempts_used < max_attempts {
                        let delay = if use_backoff {
                            backoff_delay(attempt + 1, jitter)
                        } else {
                            Duration::from_millis(0)
                        };
                        attempt += 1;
                        if !delay.is_zero() {
                            tokio::time::sleep(delay).await;
                        }
                        continue;
                    }
                    return Err(Error::Network(e));
                }
            }
        }
    }
}

fn is_safe_method(m: &Method) -> bool {
    matches!(*m, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// Parse a `Retry-After` header value into a [`Duration`]. Supports the
/// integer-seconds form; HTTP-date is left to upstream callers (the API
/// uses seconds in practice).
fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let raw = headers.get(RETRY_AFTER)?.to_str().ok()?;
    raw.trim().parse::<u64>().ok().map(Duration::from_secs)
}

fn backoff_delay(attempt: u32, jitter: bool) -> Duration {
    let base_ms: u64 = 100;
    let capped = base_ms.saturating_mul(1u64 << attempt.min(6));
    let bounded = capped.min(5_000);
    if !jitter {
        return Duration::from_millis(bounded);
    }
    // Equal-jitter strategy: half deterministic + uniform up to the other half.
    // Prevents thundering-herd retries when many clients hit the same 429.
    let half = bounded / 2;
    let rand_part: u64 = rand::rng().random_range(0..=half);
    Duration::from_millis(half + rand_part)
}

impl ClientBuilder {
    /// Set the API key (sent as `Authorization: Bearer <key>`).
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the OAuth client ID. Used by AuthKit / SSO helpers and embedded
    /// in OAuth-flow request bodies via `inferFromClient`.
    pub fn client_id(mut self, id: impl Into<String>) -> Self {
        self.client_id = Some(id.into());
        self
    }

    /// Override the API base URL. Defaults to [`DEFAULT_BASE_URL`]; mostly
    /// useful for tests with a local mock server.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Per-request timeout for the built-in `reqwest` transport. Custom
    /// transports manage their own timeouts.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Maximum number of retries on retryable failures (default 3). Only
    /// applies to GET/HEAD/OPTIONS or requests carrying an idempotency key —
    /// see [`crate::RequestStrategy`] for per-request control.
    pub fn max_retries(mut self, max: u32) -> Self {
        self.max_retries = Some(max);
        self
    }

    /// Override the `User-Agent` header (default `"workos-rust/<version>"`).
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Plug in a custom HTTP transport. Disables the default `reqwest` impl.
    pub fn transport(mut self, transport: SharedTransport) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Build the client. Without an explicit [`ClientBuilder::transport`] the
    /// `reqwest` feature must be enabled (it is by default).
    ///
    /// Invalid API keys / user-agent strings (those that cannot be turned into
    /// an HTTP header value — e.g. they contain control bytes) are silently
    /// dropped. Use [`Self::try_build`] when validating user-supplied values
    /// is important.
    pub fn build(self) -> Client {
        // Re-use the validating path and unwrap unconditionally; previously
        // we accepted invalid header bytes silently and produced a client
        // that emitted unauthenticated requests. The behaviour is preserved
        // here only when the values *are* valid; invalid bytes panic.
        match self.try_build() {
            Ok(c) => c,
            Err(e) => panic!("ClientBuilder::build: {e}"),
        }
    }

    /// Build the client, returning an [`Error::Builder`] if any caller-
    /// supplied value (API key, user-agent) cannot be encoded as an HTTP
    /// header. Prefer this over [`Self::build`] when the API key comes from
    /// untrusted input.
    pub fn try_build(self) -> Result<Client, Error> {
        let api_key = self.api_key.unwrap_or_default();
        let timeout = self.timeout.unwrap_or(DEFAULT_TIMEOUT);
        let mut headers = HeaderMap::new();
        if !api_key.is_empty() {
            let v = HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|e| Error::Builder(format!("invalid API key: {e}")))?;
            headers.insert(AUTHORIZATION, v);
        }
        let ua = self
            .user_agent
            .as_deref()
            .unwrap_or(concat!("workos-rust/", env!("CARGO_PKG_VERSION")));
        let v = HeaderValue::from_str(ua)
            .map_err(|e| Error::Builder(format!("invalid user-agent: {e}")))?;
        headers.insert(USER_AGENT, v);

        let transport = self.transport.unwrap_or_else(|| default_transport(timeout));

        Ok(Client {
            inner: Arc::new(ClientInner {
                transport,
                base_url: self
                    .base_url
                    .unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
                max_retries: self.max_retries.unwrap_or(3),
                api_key,
                client_id: self.client_id.unwrap_or_default(),
                default_headers: headers,
            }),
        })
    }
}

#[cfg(any(feature = "native-tls", feature = "rustls-tls"))]
fn default_transport(timeout: Duration) -> SharedTransport {
    Arc::new(crate::transport::ReqwestTransport::with_timeout(timeout))
}

#[cfg(not(any(feature = "native-tls", feature = "rustls-tls")))]
fn default_transport(_timeout: Duration) -> SharedTransport {
    panic!(
        "no HTTP transport configured: build with --features rustls-tls (or native-tls), \
         or supply one via ClientBuilder::transport(...)"
    );
}
