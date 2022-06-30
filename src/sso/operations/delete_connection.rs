use async_trait::async_trait;
use serde::Serialize;
use thiserror::Error;

use crate::sso::{ConnectionId, Sso};
use crate::{ResponseExtensions, WorkOsError, WorkOsResult};

/// The parameters for [`DeleteConnection`].
#[derive(Debug, Serialize)]
pub struct DeleteConnectionParams<'a> {
    /// The ID of the connection to delete.
    pub connection_id: &'a ConnectionId,
}

/// An error returned from [`DeleteConnection`].
#[derive(Debug, Error)]
pub enum DeleteConnectionError {}

impl From<DeleteConnectionError> for WorkOsError<DeleteConnectionError> {
    fn from(err: DeleteConnectionError) -> Self {
        Self::Operation(err)
    }
}

/// [WorkOS Docs: Delete a Connection](https://workos.com/docs/reference/sso/connection/delete)
#[async_trait]
pub trait DeleteConnection {
    /// Deletes a [`Connection`](crate::sso::Connection).
    ///
    /// [WorkOS Docs: Delete a Connection](https://workos.com/docs/reference/sso/connection/delete)
    async fn delete_connection(
        &self,
        params: &DeleteConnectionParams<'_>,
    ) -> WorkOsResult<(), DeleteConnectionError>;
}

#[async_trait]
impl<'a> DeleteConnection for Sso<'a> {
    async fn delete_connection(
        &self,
        params: &DeleteConnectionParams<'_>,
    ) -> WorkOsResult<(), DeleteConnectionError> {
        let url = self
            .workos
            .base_url()
            .join(&format!("/connections/{id}", id = params.connection_id))?;
        self.workos
            .client()
            .delete(url)
            .bearer_auth(self.workos.key())
            .send()
            .await?
            .ensure_successful()?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use matches::assert_matches;
    use mockito::{self, mock};
    use tokio;

    use crate::sso::ConnectionId;
    use crate::{ApiKey, WorkOs};

    use super::*;

    #[tokio::test]
    async fn it_calls_the_delete_connection_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("DELETE", "/connections/conn_01E2NPPCT7XQ2MVVYDHWGK1WN4")
            .match_header("Authorization", "Bearer sk_example_123456789")
            .with_status(202)
            .create();

        let result = workos
            .sso()
            .delete_connection(&DeleteConnectionParams {
                connection_id: &ConnectionId::from("conn_01E2NPPCT7XQ2MVVYDHWGK1WN4"),
            })
            .await;

        assert_matches!(result, Ok(()));
    }
}
