use async_trait::async_trait;
use reqwest::StatusCode;
use thiserror::Error;

use crate::sso::{Connection, ConnectionId, Sso};
use crate::{WorkOsError, WorkOsResult};

#[derive(Debug, Error)]
pub enum GetConnectionError {}

impl From<GetConnectionError> for WorkOsError<GetConnectionError> {
    fn from(err: GetConnectionError) -> Self {
        Self::Operation(err)
    }
}

#[async_trait]
pub trait GetConnection {
    /// Retrieves a [`Connection`] by its ID.
    ///
    /// [WorkOS Docs: Get a Connection](https://workos.com/docs/reference/sso/connection/get)
    async fn get_connection(
        &self,
        id: &ConnectionId,
    ) -> WorkOsResult<Connection, GetConnectionError>;
}

#[async_trait]
impl<'a> GetConnection for Sso<'a> {
    async fn get_connection(
        &self,
        id: &ConnectionId,
    ) -> WorkOsResult<Connection, GetConnectionError> {
        let url = self
            .workos
            .base_url()
            .join(&format!("/connections/{id}", id = id))?;
        let response = self
            .workos
            .client()
            .get(url)
            .bearer_auth(self.workos.key())
            .send()
            .await?;

        match response.error_for_status_ref() {
            Ok(_) => {
                let connection = response.json::<Connection>().await?;

                Ok(connection)
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
    async fn it_calls_the_get_connection_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("GET", "/connections/conn_01E4ZCR3C56J083X43JQXF3JK5")
            .match_header("Authorization", "Bearer sk_example_123456789")
            .with_status(200)
            .with_body(
                json!({
                  "object": "connection",
                  "id": "conn_01E4ZCR3C56J083X43JQXF3JK5",
                  "organization_id": "org_01EHWNCE74X7JSDV0X3SZ3KJNY",
                  "connection_type": "GoogleOAuth",
                  "name": "Foo Corp",
                  "state": "active",
                  "created_at": "2021-06-25T19:07:33.155Z",
                  "updated_at": "2021-06-25T19:07:33.155Z",
                  "domains": [
                    {
                      "id": "conn_domain_01EHWNFTAFCF3CQAE5A9Q0P1YB",
                      "object": "connection_domain",
                      "domain": "foo-corp.com"
                    }
                  ]
                })
                .to_string(),
            )
            .create();

        let connection = workos
            .sso()
            .get_connection(&ConnectionId::from("conn_01E4ZCR3C56J083X43JQXF3JK5"))
            .await
            .unwrap();

        assert_eq!(
            connection.id,
            ConnectionId::from("conn_01E4ZCR3C56J083X43JQXF3JK5")
        )
    }

    #[tokio::test]
    async fn it_returns_an_error_when_the_get_connection_endpoint_returns_unauthorized() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("GET", "/connections/conn_01E4ZCR3C56J083X43JQXF3JK5")
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
            .sso()
            .get_connection(&ConnectionId::from("conn_01E4ZCR3C56J083X43JQXF3JK5"))
            .await;

        assert_matches!(result, Err(WorkOsError::Unauthorized))
    }
}