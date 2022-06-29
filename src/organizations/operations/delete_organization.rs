use async_trait::async_trait;
use reqwest::StatusCode;
use serde::Serialize;
use thiserror::Error;

use crate::organizations::{OrganizationId, Organizations};
use crate::{WorkOsError, WorkOsResult};

/// The options for [`DeleteOrganization`].
#[derive(Debug, Serialize)]
pub struct DeleteOrganizationOptions<'a> {
    /// The ID of the organization.
    pub organization_id: &'a OrganizationId,
}

/// An error returned from [`DeleteOrganization`].
#[derive(Debug, Error)]
pub enum DeleteOrganizationError {}

impl From<DeleteOrganizationError> for WorkOsError<DeleteOrganizationError> {
    fn from(err: DeleteOrganizationError) -> Self {
        Self::Operation(err)
    }
}

/// [WorkOS Docs: Delete an Organization](https://workos.com/docs/reference/organization/delete)
#[async_trait]
pub trait DeleteOrganization {
    /// Creates an [`Organization`](crate::organizations::Organization).
    ///
    /// [WorkOS Docs: Delete an Organization](https://workos.com/docs/reference/organization/delete)
    async fn delete_organization(
        &self,
        options: &DeleteOrganizationOptions<'_>,
    ) -> WorkOsResult<(), DeleteOrganizationError>;
}

#[async_trait]
impl<'a> DeleteOrganization for Organizations<'a> {
    async fn delete_organization(
        &self,
        options: &DeleteOrganizationOptions<'_>,
    ) -> WorkOsResult<(), DeleteOrganizationError> {
        let url = self.workos.base_url().join(&format!(
            "/organizations/{id}",
            id = options.organization_id
        ))?;
        let response = self
            .workos
            .client()
            .delete(url)
            .bearer_auth(self.workos.key())
            .send()
            .await?;

        match response.error_for_status_ref() {
            Ok(_) => Ok(()),
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
    use tokio;

    use super::*;
    use crate::{ApiKey, WorkOs};
    use matches::assert_matches;

    #[tokio::test]
    async fn it_calls_the_delete_organization_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("DELETE", "/organizations/org_01EHZNVPK3SFK441A1RGBFSHRT")
            .match_header("Authorization", "Bearer sk_example_123456789")
            .with_status(202)
            .create();

        let result = workos
            .organizations()
            .delete_organization(&DeleteOrganizationOptions {
                organization_id: &OrganizationId::from("org_01EHZNVPK3SFK441A1RGBFSHRT"),
            })
            .await;

        assert_matches!(result, Ok(()));
    }
}