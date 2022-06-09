use std::collections::HashSet;

use async_trait::async_trait;
use reqwest::StatusCode;
use serde::Serialize;
use thiserror::Error;

use crate::organizations::{Organization, Organizations};
use crate::{WorkOsError, WorkOsResult};

#[derive(Debug, Serialize)]
pub struct CreateOrganizationOptions<'a> {
    /// The name of the organization.
    pub name: &'a str,

    /// Whether the connections within this organization should allow profiles
    /// that do not have a domain that is present in the set of the organization's
    /// user email domains.
    ///
    /// See [here](https://workos.com/docs/sso/guide/frequently-asked-questions#allow-profiles-outside-organization)
    /// for more details.
    pub allow_profiles_outside_organization: Option<&'a bool>,

    /// The domains of the organization.
    ///
    /// At least one domain is required unless
    /// [`allow_profiles_outside_organization`] is `true`.
    pub domains: HashSet<&'a str>,
}

#[derive(Debug, Error)]
pub enum CreateOrganizationError {}

impl From<CreateOrganizationError> for WorkOsError<CreateOrganizationError> {
    fn from(err: CreateOrganizationError) -> Self {
        Self::Operation(err)
    }
}

#[async_trait]
pub trait CreateOrganization {
    /// Creates an [`Organization`].
    ///
    /// [WorkOS Docs: Create an Organization](https://workos.com/docs/reference/organization/create)
    async fn create_organization(
        &self,
        options: &CreateOrganizationOptions<'_>,
    ) -> WorkOsResult<Organization, CreateOrganizationError>;
}

#[async_trait]
impl<'a> CreateOrganization for Organizations<'a> {
    async fn create_organization(
        &self,
        options: &CreateOrganizationOptions<'_>,
    ) -> WorkOsResult<Organization, CreateOrganizationError> {
        let url = self.workos.base_url().join("/organizations")?;
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
                let organization = response.json::<Organization>().await?;

                Ok(organization)
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
    async fn it_calls_the_create_organization_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("POST", "/organizations")
            .match_header("Authorization", "Bearer sk_example_123456789")
            .with_status(201)
            .with_body(
                json!({
                    "id": "org_01EHZNVPK3SFK441A1RGBFSHRT",
                    "object": "organization",
                    "name": "Foo Corp",
                    "allow_profiles_outside_organization": false,
                    "created_at": "2021-06-25T19:07:33.155Z",
                    "updated_at": "2021-06-25T19:07:33.155Z",
                    "domains": [
                        {
                            "domain": "foo-corp.com",
                            "id": "org_domain_01EHZNVPK2QXHMVWCEDQEKY69A",
                            "object": "organization_domain"
                        }
                    ]
                })
                .to_string(),
            )
            .create();

        let organization = workos
            .organizations()
            .create_organization(&CreateOrganizationOptions {
                name: "Foo Corp",
                allow_profiles_outside_organization: Some(&false),
                domains: HashSet::from(["foo-corp.com"]),
            })
            .await
            .unwrap();

        assert_eq!(
            organization.id,
            OrganizationId::from("org_01EHZNVPK3SFK441A1RGBFSHRT")
        )
    }
}