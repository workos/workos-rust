// @oagen-ignore-file
//! Regression coverage for VULN-1274/f053: parent scope must reach the wire,
//! including every request made by the auto-paging variants.

mod common;

use futures_util::TryStreamExt;
use serde_json::json;
use wiremock::matchers::{method, path as path_matcher, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};
use workos::authorization::{
    ListResourcesForMembershipParams, ListResourcesParams, Parent, ParentResource,
};

fn empty_page() -> serde_json::Value {
    json!({
        "object": "list",
        "data": [],
        "list_metadata": { "before": null, "after": null }
    })
}

async fn mount_two_pages(server: &MockServer, path: &str) -> Result<(), serde_json::Error> {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .and(query_param("after", "cursor_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_page()))
        .expect(1)
        .mount(server)
        .await;

    let mut first_page: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/authorization_resource_list.json"))?;
    first_page["list_metadata"] = json!({ "before": null, "after": "cursor_1" });
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .and(query_param_is_missing("after"))
        .respond_with(ResponseTemplate::new(200).set_body_json(first_page))
        .expect(1)
        .mount(server)
        .await;
    Ok(())
}

#[tokio::test]
async fn list_resources_encodes_parent_by_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_matcher("/authorization/resources"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_page()))
        .expect(1)
        .mount(&server)
        .await;
    let client = common::test_client(&server).await;
    let params = ListResourcesParams {
        parent: Some(Parent::ById {
            parent_resource_id: "res_parent_123".into(),
        }),
        ..Default::default()
    };
    let page = client
        .authorization()
        .list_resources(params)
        .await
        .expect("expected success");
    assert!(page.data.is_empty());

    let received = server.received_requests().await.expect("recorded requests");
    assert_eq!(received.len(), 1);
    let query = received[0].url.query().unwrap_or("");
    assert!(
        query
            .split('&')
            .any(|p| p == "parent_resource_id=res_parent_123"),
        "expected parent ID filter, got {query:?}"
    );
}

#[tokio::test]
async fn list_resources_encodes_parent_by_external_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_matcher("/authorization/resources"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_page()))
        .expect(1)
        .mount(&server)
        .await;
    let client = common::test_client(&server).await;
    let params = ListResourcesParams {
        parent: Some(Parent::ByExternalId {
            parent_resource_type_slug: "workspace".into(),
            parent_external_id: "workspace_123".into(),
        }),
        ..Default::default()
    };
    let page = client
        .authorization()
        .list_resources(params)
        .await
        .expect("expected success");
    assert!(page.data.is_empty());

    let received = server.received_requests().await.expect("recorded requests");
    assert_eq!(received.len(), 1);
    let query = received[0].url.query().unwrap_or("");
    for filter in [
        "parent_resource_type_slug=workspace",
        "parent_external_id=workspace_123",
    ] {
        assert!(
            query.split('&').any(|p| p == filter),
            "expected {filter:?}, got {query:?}"
        );
    }
}

#[tokio::test]
async fn list_resources_auto_paging_keeps_parent_filter_on_all_pages() {
    let server = MockServer::start().await;
    mount_two_pages(&server, "/authorization/resources")
        .await
        .expect("valid resource list fixture");
    let client = common::test_client(&server).await;
    let params = ListResourcesParams {
        parent: Some(Parent::ById {
            parent_resource_id: "res_parent_123".into(),
        }),
        ..Default::default()
    };
    let resources: Vec<_> = client
        .authorization()
        .list_resources_auto_paging(params)
        .try_collect()
        .await
        .expect("expected all pages to succeed");
    assert_eq!(resources.len(), 1);

    let received = server.received_requests().await.expect("recorded requests");
    assert_eq!(received.len(), 2);
    for (index, request) in received.iter().enumerate() {
        let query = request.url.query().unwrap_or("");
        assert!(
            query
                .split('&')
                .any(|p| p == "parent_resource_id=res_parent_123"),
            "expected parent ID filter on page {}, got {query:?}",
            index + 1
        );
        assert_eq!(
            request
                .url
                .query_pairs()
                .find(|(key, _)| key == "after")
                .map(|(_, value)| value.into_owned()),
            (index == 1).then(|| "cursor_1".to_string())
        );
    }
}

#[tokio::test]
async fn list_resources_for_membership_auto_paging_keeps_parent_filter_on_all_pages() {
    let server = MockServer::start().await;
    mount_two_pages(
        &server,
        "/authorization/organization_memberships/om_123/resources",
    )
    .await
    .expect("valid resource list fixture");
    let client = common::test_client(&server).await;
    let params = ListResourcesForMembershipParams::new(
        "perm_slug",
        ParentResource::ById {
            parent_resource_id: "res_parent_123".into(),
        },
    );
    let resources: Vec<_> = client
        .authorization()
        .list_resources_for_membership_auto_paging("om_123", params)
        .try_collect()
        .await
        .expect("expected all pages to succeed");
    assert_eq!(resources.len(), 1);

    let received = server.received_requests().await.expect("recorded requests");
    assert_eq!(received.len(), 2);
    for (index, request) in received.iter().enumerate() {
        let query = request.url.query().unwrap_or("");
        for filter in [
            "parent_resource_id=res_parent_123",
            "permission_slug=perm_slug",
        ] {
            assert!(
                query.split('&').any(|p| p == filter),
                "expected {filter:?} on page {}, got {query:?}",
                index + 1
            );
        }
        assert_eq!(
            request
                .url
                .query_pairs()
                .find(|(key, _)| key == "after")
                .map(|(_, value)| value.into_owned()),
            (index == 1).then(|| "cursor_1".to_string())
        );
    }
}

#[tokio::test]
async fn list_resources_for_membership_auto_paging_keeps_external_parent_filter_on_all_pages() {
    let server = MockServer::start().await;
    mount_two_pages(
        &server,
        "/authorization/organization_memberships/om_123/resources",
    )
    .await
    .expect("valid resource list fixture");
    let client = common::test_client(&server).await;
    let params = ListResourcesForMembershipParams::new(
        "perm_slug",
        ParentResource::ByExternalId {
            parent_resource_type_slug: "workspace".into(),
            parent_resource_external_id: "workspace_123".into(),
        },
    );
    let resources: Vec<_> = client
        .authorization()
        .list_resources_for_membership_auto_paging("om_123", params)
        .try_collect()
        .await
        .expect("expected all pages to succeed");
    assert_eq!(resources.len(), 1);

    let received = server.received_requests().await.expect("recorded requests");
    assert_eq!(received.len(), 2);
    for (index, request) in received.iter().enumerate() {
        let query = request.url.query().unwrap_or("");
        for filter in [
            "parent_resource_type_slug=workspace",
            "parent_resource_external_id=workspace_123",
            "permission_slug=perm_slug",
        ] {
            assert!(
                query.split('&').any(|p| p == filter),
                "expected {filter:?} on page {}, got {query:?}",
                index + 1
            );
        }
        assert_eq!(
            request
                .url
                .query_pairs()
                .find(|(key, _)| key == "after")
                .map(|(_, value)| value.into_owned()),
            (index == 1).then(|| "cursor_1".to_string())
        );
    }
}
