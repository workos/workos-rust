// @oagen-ignore-file

mod common;

use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn manual_sync_accepts_202() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/directories/directory_123/sync"))
        .respond_with(
            ResponseTemplate::new(202).set_body_json(serde_json::json!({"status": "queued"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = common::test_client(&server).await;

    let result = client
        .directory_sync()
        .sync_directory("directory_123")
        .await
        .unwrap();

    assert_eq!(result.status, "queued");
}

#[tokio::test]
async fn manual_sync_preserves_rejection_and_retry_information() {
    for (status, code) in [
        (409, "directory_sync_in_progress"),
        (422, "directory_sync_unsupported"),
        (429, "directory_sync_rate_limited"),
        (503, "directory_sync_disabled"),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/directories/directory_123/sync"))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("Retry-After", "120")
                    .set_body_json(serde_json::json!({
                        "code": code,
                        "message": "Not queued.",
                        "retry_after_seconds": 120
                    })),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = common::test_client(&server).await;

        let error = client
            .directory_sync()
            .sync_directory("directory_123")
            .await
            .unwrap_err();

        assert_eq!(error.status(), Some(status));
        assert_eq!(error.code(), Some(code));
        if status == 429 {
            assert_eq!(error.retry_after(), Some(Duration::from_secs(120)));
        }
    }
}
