use async_trait::async_trait;
use thiserror::Error;

use crate::organizations::{Organization, OrganizationId, Organizations};
use crate::{WorkOsError, WorkOsResult};

#[derive(Debug, Error)]
pub enum GetOrganizationError {}

impl From<GetOrganizationError> for WorkOsError<GetOrganizationError> {
    fn from(err: GetOrganizationError) -> Self {
        Self::Operation(err)
    }
}

#[async_trait]
pub trait GetOrganization {
    /// Retrieves an [`Organization`] by its ID.
    ///
    /// [WorkOS Docs: Get an Organization](https://workos.com/docs/reference/sso/organization/get)
    async fn get_organization(
        &self,
        id: &OrganizationId,
    ) -> WorkOsResult<Organization, GetOrganizationError>;
}

#[async_trait]
impl<'a> GetOrganization for Organizations<'a> {
    async fn get_organization(
        &self,
        id: &OrganizationId,
    ) -> WorkOsResult<Organization, GetOrganizationError> {
        let url = self
            .workos
            .base_url()
            .join(&format!("/organizations/{id}", id = id))?;
        let response = self
            .workos
            .client()
            .get(url)
            .bearer_auth(self.workos.key())
            .send()
            .await?;
        let organization = response.json::<Organization>().await?;

        Ok(organization)
    }
}

#[cfg(test)]
mod test {
    use mockito::{self, mock};
    use serde_json::json;
    use tokio;

    use crate::{ApiKey, WorkOs};

    use super::*;

    #[tokio::test]
    async fn it_calls_the_get_organization_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("GET", "/organizations/org_01EHZNVPK3SFK441A1RGBFSHRT")
            .match_header("Authorization", "Bearer sk_example_123456789")
            .with_status(200)
            .with_body(
                json!({
                  "id": "org_01EHZNVPK3SFK441A1RGBFSHRT",
                  "object": "organization",
                  "name": "Foo Corporation",
                  "allow_profiles_outside_organization": false,
                  "created_at": "2021-06-25T19:07:33.155Z",
                  "updated_at": "2021-06-25T19:07:33.155Z",
                  "domains": [
                    {
                      "domain": "foo-corp.com",
                      "id": "org_domain_01EHZNVPK2QXHMVWCEDQEKY69A",
                      "object": "organization_domain"
                    },
                    {
                      "domain": "another-foo-corp-domain.com",
                      "id": "org_domain_01EHZNS0H9W90A90FV79GAB6AB",
                      "object": "organization_domain"
                    }
                  ]
                })
                .to_string(),
            )
            .create();

        let organization = workos
            .organizations()
            .get_organization(&OrganizationId::from("org_01EHZNVPK3SFK441A1RGBFSHRT"))
            .await
            .unwrap();

        assert_eq!(
            organization.id,
            OrganizationId::from("org_01EHZNVPK3SFK441A1RGBFSHRT")
        )
    }
}