// @oagen-ignore-file
//! Locks in the SDK's retry-safety contract:
//!
//!   - GET requests retry on 429/5xx using the client-level `max_retries`
//!     budget, even without an idempotency key.
//!   - Mutating requests (POST/PUT/PATCH/DELETE) do *not* auto-retry unless
//!     the caller supplies an idempotency key (or an explicit
//!     `RequestStrategy`).
//!   - The `Retry-After` header overrides the default exponential backoff.
//!   - `RequestStrategy::Once` disables retries entirely.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use async_trait::async_trait;
use bytes::Bytes;
use http::{HeaderMap, HeaderValue, StatusCode, header::RETRY_AFTER};

use workos::transport::{HttpRequest, HttpResponse, HttpTransport, TransportError};
use workos::{Client, RequestOptions, RequestStrategy};

struct CountingTransport {
    calls: AtomicU32,
    status: u16,
    retry_after_seconds: Option<u64>,
}

impl CountingTransport {
    fn new(status: u16) -> Self {
        Self {
            calls: AtomicU32::new(0),
            status,
            retry_after_seconds: None,
        }
    }

    fn with_retry_after(mut self, seconds: u64) -> Self {
        self.retry_after_seconds = Some(seconds);
        self
    }

    fn calls(&self) -> u32 {
        self.calls.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl HttpTransport for CountingTransport {
    async fn execute(&self, _req: HttpRequest) -> Result<HttpResponse, TransportError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let mut headers = HeaderMap::new();
        if let Some(s) = self.retry_after_seconds {
            headers.insert(RETRY_AFTER, HeaderValue::from(s));
        }
        Ok(HttpResponse {
            status: StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            headers,
            body: Bytes::from_static(b"{}"),
        })
    }
}

fn build_client(transport: Arc<dyn HttpTransport>, max_retries: u32) -> Client {
    Client::builder()
        .api_key("k")
        .base_url("http://test.invalid")
        .max_retries(max_retries)
        .transport(transport)
        .build()
}

#[tokio::test]
async fn get_retries_on_5xx_up_to_max_retries() {
    let transport = Arc::new(CountingTransport::new(503));
    let client = build_client(transport.clone(), 2);
    // GET is idempotent → SDK retries even without an idempotency key. Use
    // any GET endpoint via the resource API: list_organizations is convenient.
    let _ = client
        .organizations()
        .list_organizations(workos::organizations::ListOrganizationsParams::default())
        .await;
    // 1 initial + 2 retries = 3 total.
    assert_eq!(transport.calls(), 3);
}

#[tokio::test]
async fn post_does_not_retry_without_idempotency_key() {
    let transport = Arc::new(CountingTransport::new(503));
    let client = build_client(transport.clone(), 3);
    let _ = client
        .organizations()
        .create_organization(workos::organizations::CreateOrganizationParams::new(
            workos::OrganizationInput {
                name: "Acme".to_string(),
                allow_profiles_outside_organization: None,
                domains: None,
                domain_data: None,
                metadata: None,
                external_id: None,
            },
        ))
        .await;
    // Mutating call without an idempotency key sends exactly once.
    assert_eq!(transport.calls(), 1);
}

#[tokio::test]
async fn post_retries_with_idempotency_key() {
    let transport = Arc::new(CountingTransport::new(503));
    let client = build_client(transport.clone(), 2);
    let opts = RequestOptions::new().idempotency_key("ik_test");
    let _ = client
        .organizations()
        .create_organization_with_options(
            workos::organizations::CreateOrganizationParams::new(workos::OrganizationInput {
                name: "Acme".to_string(),
                allow_profiles_outside_organization: None,
                domains: None,
                domain_data: None,
                metadata: None,
                external_id: None,
            }),
            Some(&opts),
        )
        .await;
    assert_eq!(transport.calls(), 3);
}

#[tokio::test]
async fn request_strategy_once_disables_retries_for_get() {
    let transport = Arc::new(CountingTransport::new(503));
    let client = build_client(transport.clone(), 5);
    let opts = RequestOptions::new().strategy(RequestStrategy::Once);
    let _ = client
        .organizations()
        .list_organizations_with_options(
            workos::organizations::ListOrganizationsParams::default(),
            Some(&opts),
        )
        .await;
    assert_eq!(transport.calls(), 1);
}

#[tokio::test]
async fn retry_after_header_is_honored() {
    // The transport keeps returning 429 with `Retry-After: 0` so the body
    // of execute() runs through the retry path without any real sleep.
    let transport = Arc::new(CountingTransport::new(429).with_retry_after(0));
    let client = build_client(transport.clone(), 2);
    let _ = client
        .organizations()
        .list_organizations(workos::organizations::ListOrganizationsParams::default())
        .await;
    // Retries still happen — `Retry-After: 0` is just shorter sleep, not a
    // retry-disable signal.
    assert_eq!(transport.calls(), 3);
}

#[tokio::test]
async fn idempotent_strategy_overrides_explicit_key() {
    let transport = Arc::new(CountingTransport::new(503));
    let client = build_client(transport.clone(), 1);
    let opts = RequestOptions::new()
        .idempotency_key("explicit_key")
        .strategy(RequestStrategy::Idempotent("strategy_key".to_string()));
    let _ = client
        .organizations()
        .create_organization_with_options(
            workos::organizations::CreateOrganizationParams::new(workos::OrganizationInput {
                name: "Acme".to_string(),
                allow_profiles_outside_organization: None,
                domains: None,
                domain_data: None,
                metadata: None,
                external_id: None,
            }),
            Some(&opts),
        )
        .await;
    // 1 initial + 1 retry from max_retries=1 with idempotent strategy.
    assert_eq!(transport.calls(), 2);
}
