use async_trait::async_trait;
use reqwest::StatusCode;
use serde::Serialize;
use thiserror::Error;

use crate::directory_sync::{DirectoryId, DirectorySync};
use crate::{WorkOsError, WorkOsResult};

/// The options for [`DeleteDirectory`].
#[derive(Debug, Serialize)]
pub struct DeleteDirectoryOptions<'a> {
    /// The ID of the directory to delete.
    pub directory_id: &'a DirectoryId,
}

/// An error returned from [`DeleteDirectory`].
#[derive(Debug, Error)]
pub enum DeleteDirectoryError {}

impl From<DeleteDirectoryError> for WorkOsError<DeleteDirectoryError> {
    fn from(err: DeleteDirectoryError) -> Self {
        Self::Operation(err)
    }
}

/// [WorkOS Docs: Delete a Directory](https://workos.com/docs/reference/directory-sync/directory/delete)
#[async_trait]
pub trait DeleteDirectory {
    /// Deletes a [`Directory`](crate::directory_sync::Directory).
    ///
    /// [WorkOS Docs: Delete a Directory](https://workos.com/docs/reference/directory-sync/directory/delete)
    async fn delete_directory(
        &self,
        options: &DeleteDirectoryOptions<'_>,
    ) -> WorkOsResult<(), DeleteDirectoryError>;
}

#[async_trait]
impl<'a> DeleteDirectory for DirectorySync<'a> {
    async fn delete_directory(
        &self,
        options: &DeleteDirectoryOptions<'_>,
    ) -> WorkOsResult<(), DeleteDirectoryError> {
        let url = self
            .workos
            .base_url()
            .join(&format!("/directories/{id}", id = options.directory_id))?;
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
    use matches::assert_matches;
    use mockito::{self, mock};
    use tokio;

    use crate::directory_sync::DirectoryId;
    use crate::{ApiKey, WorkOs};

    use super::*;

    #[tokio::test]
    async fn it_calls_the_delete_directory_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock(
            "DELETE",
            "/directories/directory_01ECAZ4NV9QMV47GW873HDCX74",
        )
        .match_header("Authorization", "Bearer sk_example_123456789")
        .with_status(202)
        .create();

        let result = workos
            .directory_sync()
            .delete_directory(&DeleteDirectoryOptions {
                directory_id: &DirectoryId::from("directory_01ECAZ4NV9QMV47GW873HDCX74"),
            })
            .await;

        assert_matches!(result, Ok(()));
    }
}
