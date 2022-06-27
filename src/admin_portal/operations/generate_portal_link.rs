use async_trait::async_trait;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::admin_portal::AdminPortal;
use crate::organizations::OrganizationId;
use crate::{WorkOsError, WorkOsResult};

/// The intent of an Admin Portal session.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdminPortalIntent {
    /// The Admin Portal will be used to setup Single Sign-On (SSO).
    Sso,

    /// The Admin Portal wil be used to setup Directory Sync.
    #[serde(rename = "dsync")]
    DirectorySync,
}

/// The target of the Admin Portal.
#[derive(Debug, Serialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum AdminPortalTarget {
    /// The Admin Portal session should target an organization.
    Organization {
        /// The ID of the organization.
        #[serde(rename = "organization")]
        organization_id: OrganizationId,

        /// The intent of the Admin Portal session.
        intent: AdminPortalIntent,
    },
}

/// The options for [`GeneratePortalLink`].
#[derive(Debug, Serialize)]
pub struct GeneratePortalLinkOptions<'a> {
    /// The target of the Admin Portal.
    #[serde(flatten)]
    pub target: &'a AdminPortalTarget,

    /// The URL to which the Admin Portal should send users when they click on the link
    /// to return to your application.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_url: Option<String>,
}

/// The response for [`GeneratePortalLink`].
#[derive(Debug, Deserialize)]
pub struct GeneratePortalLinkResponse {
    /// The generate Admin Portal link.
    pub link: String,
}

/// An error from [`GeneratePortalLink`].
#[derive(Debug)]
pub enum GeneratePortalLinkError {}

/// [WorkOS Docs: Generate a Portal Link](https://workos.com/docs/reference/admin-portal/portal-link/generate)
#[async_trait]
pub trait GeneratePortalLink {
    /// Creates an [`Organization`].
    ///
    /// [WorkOS Docs: Generate a Portal Link](https://workos.com/docs/reference/admin-portal/portal-link/generate)
    async fn generate_portal_link(
        &self,
        options: &GeneratePortalLinkOptions<'_>,
    ) -> WorkOsResult<GeneratePortalLinkResponse, GeneratePortalLinkError>;
}

#[async_trait]
impl<'a> GeneratePortalLink for AdminPortal<'a> {
    async fn generate_portal_link(
        &self,
        options: &GeneratePortalLinkOptions<'_>,
    ) -> WorkOsResult<GeneratePortalLinkResponse, GeneratePortalLinkError> {
        let url = self.workos.base_url().join("/portal/generate_link")?;
        let response = self
            .workos
            .client()
            .post(url)
            .bearer_auth(self.workos.key())
            .json(&options)
            .send()
            .await?;

        match response.error_for_status_ref() {
            Ok(_) => {
                let generate_link_response = response.json::<GeneratePortalLinkResponse>().await?;

                Ok(generate_link_response)
            }
            Err(err) => match err.status() {
                Some(StatusCode::UNAUTHORIZED) => Err(WorkOsError::Unauthorized),
                _ => Err(WorkOsError::RequestError(err)),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use mockito::{self, mock};
    use serde_json::json;
    use tokio;

    use crate::{organizations::OrganizationId, ApiKey, WorkOs};

    use super::*;

    #[tokio::test]
    async fn it_calls_the_generate_portal_link_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&dbg!(mockito::server_url()))
            .unwrap()
            .build();

        let _mock = mock("POST", "/portal/generate_link")
            .match_header("Authorization", "Bearer sk_example_123456789")
            .match_body(r#"{"organization":"org_01EHZNVPK3SFK441A1RGBFSHRT","intent":"sso"}"#)
            .with_status(201)
            .with_body(
                json!({
                    "link": "https://setup.workos.com/portal/launch?secret=JteZqfJZqUcgWGaYCC6iI0gW0"
                })
                .to_string(),
            )
            .create();

        let GeneratePortalLinkResponse { link } = workos
            .admin_portal()
            .generate_portal_link(&GeneratePortalLinkOptions {
                target: &AdminPortalTarget::Organization {
                    organization_id: OrganizationId::from("org_01EHZNVPK3SFK441A1RGBFSHRT"),
                    intent: AdminPortalIntent::Sso,
                },
                return_url: None,
            })
            .await
            .unwrap();

        assert_eq!(
            link,
            "https://setup.workos.com/portal/launch?secret=JteZqfJZqUcgWGaYCC6iI0gW0".to_string()
        )
    }
}
