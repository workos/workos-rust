use async_trait::async_trait;
use serde::Serialize;

use crate::organizations::OrganizationId;
use crate::sso::{Connection, ConnectionType, Sso};
use crate::{KnownOrUnknown, PaginatedList, PaginationParams, ResponseExtensions, WorkOsResult};

/// The parameters for [`ListConnections`].
#[derive(Debug, Serialize)]
pub struct ListConnectionsParams<'a> {
    /// The pagination parameters to use when listing connections.
    #[serde(flatten)]
    pub pagination: PaginationParams<'a>,

    /// The ID of the organization to list connections for.
    pub organization_id: Option<&'a OrganizationId>,

    /// The type of connections to list.
    #[serde(rename = "connection_type")]
    pub r#type: &'a Option<KnownOrUnknown<ConnectionType, String>>,
}

impl<'a> Default for ListConnectionsParams<'a> {
    fn default() -> Self {
        Self {
            pagination: PaginationParams::default(),
            organization_id: None,
            r#type: &None,
        }
    }
}

/// [WorkOS Docs: List Connections](https://workos.com/docs/reference/sso/connection/list)
#[async_trait]
pub trait ListConnections {
    /// Retrieves a list of [`Connection`]s.
    ///
    /// [WorkOS Docs: List Connections](https://workos.com/docs/reference/sso/connection/list)
    async fn list_connections(
        &self,
        params: &ListConnectionsParams<'_>,
    ) -> WorkOsResult<PaginatedList<Connection>, ()>;
}

#[async_trait]
impl<'a> ListConnections for Sso<'a> {
    async fn list_connections(
        &self,
        params: &ListConnectionsParams<'_>,
    ) -> WorkOsResult<PaginatedList<Connection>, ()> {
        let url = self.workos.base_url().join("/connections")?;
        let paginated_connections = self
            .workos
            .client()
            .get(url)
            .query(&params)
            .bearer_auth(self.workos.key())
            .send()
            .await?
            .handle_unauthorized()?
            .json::<PaginatedList<Connection>>()
            .await?;

        Ok(paginated_connections)
    }
}

#[cfg(test)]
mod test {
    use mockito::{self, mock, Matcher};
    use serde_json::json;
    use tokio;

    use crate::{sso::ConnectionId, ApiKey, WorkOs};

    use super::*;

    #[tokio::test]
    async fn it_calls_the_list_connections_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("GET", "/connections")
            .match_query(Matcher::UrlEncoded("order".to_string(), "desc".to_string()))
            .match_header("Authorization", "Bearer sk_example_123456789")
            .with_status(200)
            .with_body(
                json!({
                  "data": [
                    {
                      "object": "connection",
                      "id": "conn_01E4ZCR3C56J083X43JQXF3JK5",
                      "organization_id": "org_01EHWNCE74X7JSDV0X3SZ3KJNY",
                      "connection_type": "GoogleOAuth",
                      "name": "Foo Corp",
                      "state": "active",
                      "created_at": "2021-06-25T19:07:33.155Z",
                      "updated_at": "2021-06-25T19:08:33.155Z"
                    },
                    {
                      "object": "connection",
                      "id": "conn_01E2NPPCT7XQ2MVVYDHWGK1WN4",
                      "organization_id": "org_01EHWNCE74X7JSDV0X3SZ3KJNY",
                      "connection_type": "OktaSAML",
                      "name": "Example Co",
                      "state": "active",
                      "created_at": "2021-06-25T19:09:33.155Z",
                      "updated_at": "2021-06-25T19:10:33.155Z"
                    }
                  ],
                  "list_metadata": {
                    "after": "conn_01E2NPPCT7XQ2MVVYDHWGK1WN4",
                    "before": null
                  }
                })
                .to_string(),
            )
            .create();

        let paginated_list = workos
            .sso()
            .list_connections(&Default::default())
            .await
            .unwrap();

        assert_eq!(
            paginated_list.metadata.after,
            Some("conn_01E2NPPCT7XQ2MVVYDHWGK1WN4".to_string())
        )
    }

    #[tokio::test]
    async fn it_calls_the_list_connections_endpoint_with_the_connection_type() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("GET", "/connections")
            .match_query(Matcher::UrlEncoded(
                "connection_type".to_string(),
                "OktaSAML".to_string(),
            ))
            .match_header("Authorization", "Bearer sk_example_123456789")
            .with_status(200)
            .with_body(
                json!({
                  "data": [
                    {
                      "object": "connection",
                      "id": "conn_01E2NPPCT7XQ2MVVYDHWGK1WN4",
                      "organization_id": "org_01EHWNCE74X7JSDV0X3SZ3KJNY",
                      "connection_type": "OktaSAML",
                      "name": "Example Co",
                      "state": "active",
                      "created_at": "2021-06-25T19:09:33.155Z",
                      "updated_at": "2021-06-25T19:10:33.155Z"
                    }
                  ],
                  "list_metadata": {
                    "after": "conn_01E2NPPCT7XQ2MVVYDHWGK1WN4",
                    "before": null
                  }
                })
                .to_string(),
            )
            .create();

        let paginated_list = workos
            .sso()
            .list_connections(&ListConnectionsParams {
                r#type: &Some(KnownOrUnknown::Known(ConnectionType::OktaSaml)),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(
            paginated_list
                .data
                .into_iter()
                .next()
                .map(|connection| connection.id),
            Some(ConnectionId::from("conn_01E2NPPCT7XQ2MVVYDHWGK1WN4"))
        )
    }
}
