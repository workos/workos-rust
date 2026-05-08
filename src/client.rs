// @oagen-ignore-file
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, Method, header::AUTHORIZATION, header::USER_AGENT};
use serde::{Serialize, de::DeserializeOwned};

use crate::error::Error;
use crate::transport::{HttpRequest, SharedTransport};

/// Default base URL. Override via [`ClientBuilder::base_url`].
pub const DEFAULT_BASE_URL: &str = "https://api.workos.com";

/// Default request timeout (used by the built-in `reqwest` transport).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

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
    /// Requires the `reqwest` feature (default-on). For custom transports use
    /// [`Client::builder`] with [`ClientBuilder::transport`].
    #[cfg(feature = "reqwest")]
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
        let req = self.build_request(method, path, Some(params), None::<&()>)?;
        self.send(req).await
    }

    pub(crate) async fn request_with_body<P: Serialize, B: Serialize, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        params: &P,
        body: Option<&B>,
    ) -> Result<R, Error> {
        let req = self.build_request(method, path, Some(params), body)?;
        self.send(req).await
    }

    /// POST/PUT a JSON body and deserialize the response.
    pub(crate) async fn request_json<B: Serialize, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: &B,
    ) -> Result<R, Error> {
        let req = self.build_request(method, path, None::<&()>, Some(body))?;
        self.send(req).await
    }

    /// POST/PUT/DELETE/GET that does not deserialize a response body.
    pub(crate) async fn request_empty<B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<(), Error> {
        let req = self.build_request(method, path, None::<&()>, body)?;
        self.send_no_body(req).await
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
    ) -> Result<HttpRequest, Error> {
        let mut url = format!("{}{}", self.inner.base_url, path);
        if let Some(p) = query {
            let qs = serde_urlencoded::to_string(p)
                .map_err(|e| Error::Builder(format!("query encode failed: {e}")))?;
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

        Ok(HttpRequest {
            method,
            url,
            headers,
            body: body_bytes,
        })
    }

    async fn send_no_body(&self, req: HttpRequest) -> Result<(), Error> {
        let resp = self.execute_with_retry(req).await?;
        let status = resp.status.as_u16();
        if (200..300).contains(&status) {
            Ok(())
        } else {
            Err(Error::Api {
                status,
                code: None,
                message: String::from_utf8_lossy(&resp.body).into_owned(),
            })
        }
    }

    async fn send<R: DeserializeOwned>(&self, req: HttpRequest) -> Result<R, Error> {
        let resp = self.execute_with_retry(req).await?;
        let status = resp.status.as_u16();
        if (200..300).contains(&status) {
            serde_json::from_slice::<R>(&resp.body).map_err(Error::from)
        } else {
            Err(Error::Api {
                status,
                code: None,
                message: String::from_utf8_lossy(&resp.body).into_owned(),
            })
        }
    }

    async fn execute_with_retry(
        &self,
        req: HttpRequest,
    ) -> Result<crate::transport::HttpResponse, Error> {
        let mut attempt: u32 = 0;
        loop {
            let cloned = req.clone();
            let result = self.inner.transport.execute(cloned).await;
            match result {
                Ok(resp) => {
                    let status = resp.status.as_u16();
                    let retryable = status == 429 || (500..=599).contains(&status);
                    if retryable && attempt < self.inner.max_retries {
                        attempt += 1;
                        tokio::time::sleep(backoff_delay(attempt)).await;
                        continue;
                    }
                    return Ok(resp);
                }
                Err(e) => {
                    if e.is_retryable() && attempt < self.inner.max_retries {
                        attempt += 1;
                        tokio::time::sleep(backoff_delay(attempt)).await;
                        continue;
                    }
                    return Err(Error::Network(e));
                }
            }
        }
    }
}

fn backoff_delay(attempt: u32) -> Duration {
    let base_ms: u64 = 100;
    let capped = base_ms.saturating_mul(1u64 << attempt.min(6));
    Duration::from_millis(capped.min(5_000))
}

impl ClientBuilder {
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn client_id(mut self, id: impl Into<String>) -> Self {
        self.client_id = Some(id.into());
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn max_retries(mut self, max: u32) -> Self {
        self.max_retries = Some(max);
        self
    }

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
    pub fn build(self) -> Client {
        let api_key = self.api_key.unwrap_or_default();
        let timeout = self.timeout.unwrap_or(DEFAULT_TIMEOUT);
        let mut headers = HeaderMap::new();
        if !api_key.is_empty()
            && let Ok(v) = HeaderValue::from_str(&format!("Bearer {api_key}"))
        {
            headers.insert(AUTHORIZATION, v);
        }
        let ua = self.user_agent.as_deref().unwrap_or("workos-rust");
        if let Ok(v) = HeaderValue::from_str(ua) {
            headers.insert(USER_AGENT, v);
        }

        let transport = self.transport.unwrap_or_else(|| default_transport(timeout));

        Client {
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
        }
    }
}

#[cfg(feature = "reqwest")]
fn default_transport(timeout: Duration) -> SharedTransport {
    Arc::new(crate::transport::ReqwestTransport::with_timeout(timeout))
}

#[cfg(not(feature = "reqwest"))]
fn default_transport(_timeout: Duration) -> SharedTransport {
    panic!(
        "no HTTP transport configured: build with --features reqwest, or supply one via \
         ClientBuilder::transport(...)"
    );
}
