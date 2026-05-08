use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use workos::client::Client;
use workos::transport::{HttpRequest, HttpResponse, HttpTransport, TransportError};

#[derive(Default)]
struct RecordingTransport {
    calls: AtomicUsize,
    last_url: tokio::sync::Mutex<Option<String>>,
    last_method: tokio::sync::Mutex<Option<http::Method>>,
}

#[async_trait]
impl HttpTransport for RecordingTransport {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, TransportError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.last_url.lock().await = Some(req.url.clone());
        *self.last_method.lock().await = Some(req.method.clone());
        let body = br#"{"object":"list","data":[],"list_metadata":{"before":null,"after":null}}"#;
        Ok(HttpResponse {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: Bytes::from_static(body),
        })
    }
}

#[tokio::test]
async fn custom_transport_replaces_reqwest() {
    let transport = Arc::new(RecordingTransport::default());
    let client = Client::builder()
        .api_key("sk_test_dummy")
        .base_url("https://example.test")
        .transport(transport.clone())
        .build();

    let _ = client
        .organizations()
        .list_organizations(Default::default())
        .await
        .expect("call should succeed via the custom transport");

    assert_eq!(transport.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        transport.last_url.lock().await.as_deref(),
        Some("https://example.test/organizations")
    );
    assert_eq!(
        transport.last_method.lock().await.as_ref(),
        Some(&http::Method::GET)
    );
}
