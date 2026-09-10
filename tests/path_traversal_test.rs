// @oagen-ignore-file
//! Reject path parameters that would normalize onto a different API endpoint.

mod common;

use wiremock::matchers::{any, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use workos::Error;
use workos::authorization::AddOrganizationRolePermissionParams;
use workos::models::AddRolePermission;

#[tokio::test]
async fn invalid_path_parameters_never_send_requests() {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let client = common::test_client(&server).await;

    for segment in ["..", ".", ""] {
        let result = client
            .authorization()
            .add_organization_role_permission(
                segment,
                "admin",
                AddOrganizationRolePermissionParams::new(AddRolePermission {
                    slug: "documents:write".into(),
                }),
            )
            .await;
        assert!(matches!(result, Err(Error::Builder(_))));

        let result = client.user_management().get_user(segment).await;
        assert!(matches!(result, Err(Error::Builder(_))));

        let result = client
            .user_management()
            .find_invitation_by_token(segment)
            .await;
        assert!(matches!(result, Err(Error::Builder(_))));

        let result = client
            .groups()
            .delete_organization_group("org_1", segment)
            .await;
        assert!(matches!(result, Err(Error::Builder(_))));

        let result = client.passwordless().send_session(segment).await;
        assert!(matches!(result, Err(Error::Builder(_))));
    }
    server.verify().await;
}

#[tokio::test]
async fn dotted_user_id_preserves_request_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user_management/users/user_01ABC.x"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(include_str!("fixtures/user.json")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = common::test_client(&server).await;

    let result = client.user_management().get_user("user_01ABC.x").await;
    assert!(
        result.is_ok(),
        "expected successful user response: {result:?}"
    );
    server.verify().await;
}
