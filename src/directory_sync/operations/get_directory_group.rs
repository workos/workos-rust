use async_trait::async_trait;
use reqwest::StatusCode;
use thiserror::Error;

use crate::directory_sync::{DirectoryGroup, DirectoryGroupId, DirectorySync};
use crate::{WorkOsError, WorkOsResult};

#[derive(Debug, Error)]
pub enum GetDirectoryError {}

impl From<GetDirectoryError> for WorkOsError<GetDirectoryError> {
    fn from(err: GetDirectoryError) -> Self {
        Self::Operation(err)
    }
}

#[async_trait]
pub trait GetDirectoryGroup {
    /// Retrieves a [`DirectoryGroup`] by its ID.
    ///
    /// [WorkOS Docs: Get a Directory Group](https://workos.com/docs/reference/directory-sync/group/get)
    async fn get_directory(
        &self,
        id: &DirectoryGroupId,
    ) -> WorkOsResult<DirectoryGroup, GetDirectoryError>;
}

#[async_trait]
impl<'a> GetDirectoryGroup for DirectorySync<'a> {
    async fn get_directory(
        &self,
        id: &DirectoryGroupId,
    ) -> WorkOsResult<DirectoryGroup, GetDirectoryError> {
        let url = self
            .workos
            .base_url()
            .join(&format!("/directory_groups/{id}", id = id))?;
        let response = self
            .workos
            .client()
            .get(url)
            .bearer_auth(self.workos.key())
            .send()
            .await?;

        match response.error_for_status_ref() {
            Ok(_) => {
                let directory = response.json::<DirectoryGroup>().await?;

                Ok(directory)
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
    use matches::assert_matches;
    use mockito::{self, mock};
    use serde_json::json;
    use tokio;

    use crate::{ApiKey, WorkOs};

    use super::*;

    #[tokio::test]
    async fn it_calls_the_get_directory_group_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock(
            "GET",
            "/directory_groups/directory_group_01E64QTDNS0EGJ0FMCVY9BWGZT",
        )
        .match_header("Authorization", "Bearer sk_example_123456789")
        .with_status(200)
        .with_body(
            json!({
              "id" : "directory_group_01E64QTDNS0EGJ0FMCVY9BWGZT",
              "idp_id": "02grqrue4294w24",
              "directory_id": "directory_01ECAZ4NV9QMV47GW873HDCX74",
              "name" : "Developers",
              "created_at": "2021-06-25T19:07:33.155Z",
              "updated_at": "2021-06-25T19:07:33.155Z",
              "raw_attributes": {"directory_group_id" : "directory_group_01E64QTDNS0EGJ0FMCVY9BWGZT"}
            })
            .to_string(),
        )
        .create();

        let directory = workos
            .directory_sync()
            .get_directory(&DirectoryGroupId::from(
                "directory_group_01E64QTDNS0EGJ0FMCVY9BWGZT",
            ))
            .await
            .unwrap();

        assert_eq!(
            directory.id,
            DirectoryGroupId::from("directory_group_01E64QTDNS0EGJ0FMCVY9BWGZT")
        )
    }

    #[tokio::test]
    async fn it_returns_an_error_when_the_get_directory_group_endpoint_returns_unauthorized() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock(
            "GET",
            "/directory_groups/directory_group_01E64QTDNS0EGJ0FMCVY9BWGZT",
        )
        .match_header("Authorization", "Bearer sk_example_123456789")
        .with_status(401)
        .with_body(
            json!({
                "message": "Unauthorized"
            })
            .to_string(),
        )
        .create();

        let result = workos
            .directory_sync()
            .get_directory(&DirectoryGroupId::from(
                "directory_group_01E64QTDNS0EGJ0FMCVY9BWGZT",
            ))
            .await;

        assert_matches!(result, Err(WorkOsError::Unauthorized))
    }
}
