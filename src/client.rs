// @oagen-ignore-file
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use serde::{Serialize, de::DeserializeOwned};

use crate::error::Error;

/// Default base URL. Override via [`ClientBuilder::base_url`].
pub const DEFAULT_BASE_URL: &str = "https://api.workos.com";

/// Default request timeout.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

pub(crate) struct ClientInner {
    pub(crate) http: reqwest::Client,
    pub(crate) base_url: String,
    pub(crate) max_retries: u32,
}

#[derive(Default)]
pub struct ClientBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    max_retries: Option<u32>,
    user_agent: Option<String>,
}

impl Client {
    /// Construct a new client with the given API key and default settings.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::builder().api_key(api_key).build()
    }

    /// Begin building a client with custom configuration.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    pub(crate) async fn request_with_query<P: Serialize, R: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        params: &P,
    ) -> Result<R, Error> {
        let url = format!("{}{}", self.inner.base_url, path);
        let req = self
            .inner
            .http
            .request(method, &url)
            .query(params)
            .build()
            .map_err(Error::from)?;
        self.send(req).await
    }

    pub(crate) async fn request_with_body<P: Serialize, B: Serialize, R: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        params: &P,
        body: Option<&B>,
    ) -> Result<R, Error> {
        let url = format!("{}{}", self.inner.base_url, path);
        let mut builder = self.inner.http.request(method, &url).query(params);
        if let Some(b) = body {
            builder = builder.json(b);
        }
        let req = builder.build().map_err(Error::from)?;
        self.send(req).await
    }

    async fn send<R: DeserializeOwned>(&self, req: reqwest::Request) -> Result<R, Error> {
        let mut attempt: u32 = 0;
        loop {
            let cloned = req
                .try_clone()
                .ok_or_else(|| Error::Builder("request cannot be cloned for retry".to_string()))?;
            let resp = self.inner.http.execute(cloned).await;
            match resp {
                Ok(r) if r.status().is_success() => {
                    return r.json::<R>().await.map_err(Error::from);
                }
                Ok(r) => {
                    let status = r.status().as_u16();
                    let retryable = status == 429 || (500..=599).contains(&status);
                    if retryable && attempt < self.inner.max_retries {
                        attempt += 1;
                        let delay = backoff_delay(attempt);
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    let body = r.text().await.unwrap_or_default();
                    return Err(Error::Api {
                        status,
                        code: None,
                        message: body,
                    });
                }
                Err(e) => {
                    let retryable = e.is_connect() || e.is_timeout();
                    if retryable && attempt < self.inner.max_retries {
                        attempt += 1;
                        let delay = backoff_delay(attempt);
                        tokio::time::sleep(delay).await;
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

    pub fn build(self) -> Client {
        let api_key = self.api_key.unwrap_or_default();
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

        let http = reqwest::Client::builder()
            .timeout(self.timeout.unwrap_or(DEFAULT_TIMEOUT))
            .default_headers(headers)
            .build()
            .expect("reqwest client builder failed");

        Client {
            inner: Arc::new(ClientInner {
                http,
                base_url: self
                    .base_url
                    .unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
                max_retries: self.max_retries.unwrap_or(3),
            }),
        }
    }
}
