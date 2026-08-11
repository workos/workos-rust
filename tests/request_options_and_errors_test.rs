//! Integration tests for the hand-written request-options and error-metadata
//! plumbing. These complement the auto-generated smoke tests by actually
//! issuing requests through the live `reqwest` transport against a mock
//! `wiremock` server, then asserting on headers and parsed error fields.

mod common;

use http::{HeaderName, HeaderValue};
use wiremock::matchers::{header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use workos::{Error, RequestOptions};

// `OrganizationInput::domains` is deprecated in the spec, but struct literals
// must name every field, so tests constructing the input allow the lint.
#[allow(deprecated)]
#[tokio::test]
async fn idempotency_key_is_sent_as_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/organizations"))
        .and(header("idempotency-key", "ik_test_42"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"object":"organization","id":"org_abc","name":"Acme","external_id":null,"allow_profiles_outside_organization":false,"created_at":"2026-01-01T00:00:00.000Z","updated_at":"2026-01-01T00:00:00.000Z","domains":[],"metadata":{}}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let client = common::test_client(&server).await;
    let opts = RequestOptions::new().idempotency_key("ik_test_42");
    let body = workos::OrganizationInput {
        name: "Acme".to_string(),
        allow_profiles_outside_organization: None,
        domains: None,
        domain_data: None,
        external_id: None,
        metadata: None,
    };
    let params = workos::organizations::CreateOrganizationParams::new(body);
    let _ = client
        .organizations()
        .create_organization_with_options(params, Some(&opts))
        .await
        .expect("request should succeed");
    // wiremock asserts on drop that .expect(1) was met for the matched mock.
}

#[tokio::test]
async fn extra_headers_merge_into_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/organizations"))
        .and(header_exists("x-trace-id"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"object":"list","data":[],"list_metadata":{"before":null,"after":null}}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let client = common::test_client(&server).await;
    let opts = RequestOptions::new().header(
        HeaderName::from_static("x-trace-id"),
        HeaderValue::from_static("trace-abc"),
    );
    let params = workos::organizations::ListOrganizationsParams::default();
    let _ = client
        .organizations()
        .list_organizations_with_options(params, Some(&opts))
        .await
        .expect("request should succeed");
}

#[tokio::test]
async fn api_error_carries_structured_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/organizations/org_does_not_exist"))
        .respond_with(
            ResponseTemplate::new(404)
                .insert_header("x-request-id", "req_test_123")
                .insert_header("retry-after", "5")
                .set_body_string(r#"{"code":"organization_not_found","message":"No org"}"#),
        )
        .mount(&server)
        .await;

    let client = common::test_client(&server).await;
    let err = client
        .organizations()
        .get_organization("org_does_not_exist")
        .await
        .expect_err("expected API error");

    assert!(err.is_not_found());
    assert_eq!(err.status(), Some(404));
    assert_eq!(err.code(), Some("organization_not_found"));
    assert_eq!(err.request_id(), Some("req_test_123"));
    assert_eq!(err.retry_after(), Some(std::time::Duration::from_secs(5)));

    let api = err.api().expect("api error variant");
    assert_eq!(api.message, "No org");
    assert!(matches!(&err, Error::Api(_)));
}

#[tokio::test]
async fn api_error_falls_back_to_raw_body_for_non_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/organizations/org_x"))
        .respond_with(ResponseTemplate::new(502).set_body_string("Bad Gateway"))
        .mount(&server)
        .await;

    let client = common::test_client(&server).await;
    let err = client
        .organizations()
        .get_organization("org_x")
        .await
        .expect_err("expected API error");

    assert!(err.is_server_error());
    assert_eq!(err.code(), None);
    let api = err.api().unwrap();
    assert_eq!(api.message, "Bad Gateway");
    assert_eq!(&api.raw_body[..], b"Bad Gateway");
}
