// @oagen-ignore-file
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use workos::transport::{HttpRequest, HttpResponse, HttpTransport, TransportError};

struct Transport;

#[async_trait]
impl HttpTransport for Transport {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
        assert_eq!(request.method, http::Method::GET);
        assert!(
            request
                .url
                .starts_with("https://example.test/organizations?")
        );
        Ok(HttpResponse {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: Bytes::from_static(
                br#"{"object":"list","data":[],"list_metadata":{"before":null,"after":null}}"#,
            ),
        })
    }
}

pub async fn list_organizations() -> Result<(), workos::Error> {
    let client = workos::Client::builder()
        .api_key("test-only")
        .base_url("https://example.test")
        .max_retries(0)
        .transport(Arc::new(Transport))
        .build();
    let page = client
        .organizations()
        .list_organizations(Default::default())
        .await?;
    assert!(page.data.is_empty());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::list_organizations;

    #[tokio::test]
    async fn custom_transport_round_trip() {
        list_organizations().await.expect("transport probe failed");
    }
}
