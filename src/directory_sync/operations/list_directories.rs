use async_trait::async_trait;
use reqwest::StatusCode;
use serde::Serialize;

use crate::directory_sync::{Directory, DirectorySync, DirectoryType};
use crate::organizations::OrganizationId;
use crate::{KnownOrUnknown, PaginatedList, PaginationOptions, WorkOsError, WorkOsResult};

/// The options for [`ListDirectories`].
#[derive(Debug, Serialize)]
pub struct ListDirectoriesOptions<'a> {
    /// The domain of a directory.
    pub domain: Option<&'a String>,

    /// Searchable text to match against Directory names.
    pub search: Option<&'a String>,

    /// The pagination options to use when listing directories.
    #[serde(flatten)]
    pub pagination: PaginationOptions<'a>,

    /// The ID of the organization to list directories for.
    pub organization_id: Option<&'a OrganizationId>,

    /// The type of directories to list.
    #[serde(rename = "directory_type")]
    pub r#type: &'a Option<KnownOrUnknown<DirectoryType, String>>,
}

impl<'a> Default for ListDirectoriesOptions<'a> {
    fn default() -> Self {
        Self {
            pagination: PaginationOptions::default(),
            organization_id: None,
            r#type: &None,
            domain: None,
            search: None,
        }
    }
}

/// [WorkOS Docs: List Directories](https://workos.com/docs/reference/directory-sync/directory/list)
#[async_trait]
pub trait ListDirectories {
    /// Retrieves a list of [`Directory`]s.
    ///
    /// [WorkOS Docs: List Directories](https://workos.com/docs/reference/directory-sync/directory/list)
    async fn list_directories(
        &self,
        options: &ListDirectoriesOptions<'_>,
    ) -> WorkOsResult<PaginatedList<Directory>, ()>;
}

#[async_trait]
impl<'a> ListDirectories for DirectorySync<'a> {
    async fn list_directories(
        &self,
        options: &ListDirectoriesOptions<'_>,
    ) -> WorkOsResult<PaginatedList<Directory>, ()> {
        let url = self.workos.base_url().join("/directories")?;
        let response = self
            .workos
            .client()
            .get(url)
            .query(&options)
            .bearer_auth(self.workos.key())
            .send()
            .await?;

        match response.error_for_status_ref() {
            Ok(_) => {
                let list_directories_response = response.json::<PaginatedList<Directory>>().await?;

                Ok(list_directories_response)
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
    use mockito::{self, mock, Matcher};
    use serde_json::json;
    use tokio;

    use crate::{directory_sync::DirectoryId, ApiKey, WorkOs};

    use super::*;

    #[tokio::test]
    async fn it_calls_the_list_directories_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("GET", "/directories")
            .match_query(Matcher::UrlEncoded("order".to_string(), "desc".to_string()))
            .match_header("Authorization", "Bearer sk_example_123456789")
            .with_status(200)
            .with_body(
                json!({
                  "data": [{
                    "id": "directory_01ECAZ4NV9QMV47GW873HDCX74",
                    "domain": "foo-corp.com",
                    "name": "Foo Corp",
                    "organization_id": "org_01EHZNVPK3SFK441A1RGBFSHRT",
                    "object": "directory",
                    "state": "unlinked",
                    "type": "gsuite directory",
                    "created_at": "2021-06-25T19:07:33.155Z",
                    "updated_at": "2021-06-25T19:08:33.155Z"
                  },
                  {
                    "id": "directory_01E8CS3GSBEBZ1F1CZAEE3KHDG",
                    "domain": "foo-corp.com",
                    "external_key": "r3NDlInUnAe6i4wG",
                    "name": "Foo Corp",
                    "organization_id": "org_01EHZNVPK3SFK441A1RGBFPANT",
                    "object": "directory",
                    "state": "linked",
                    "type": "okta scim v2.0",
                    "created_at": "2021-06-25T19:09:33.155Z",
                    "updated_at": "2021-06-25T19:10:33.155Z"
                  }],
                  "list_metadata" : {
                    "after" : "directory_01E1JJS84MFPPQ3G655FHTKX6Z",
                    "before" : "directory_01E1JJS84MFPPQ3G655FHTKX6Z"
                  }
                })
                .to_string(),
            )
            .create();

        let paginated_list = workos
            .directory_sync()
            .list_directories(&Default::default())
            .await
            .unwrap();

        assert_eq!(
            paginated_list.metadata.after,
            Some("directory_01E1JJS84MFPPQ3G655FHTKX6Z".to_string())
        )
    }

    #[tokio::test]
    async fn it_calls_the_list_directories_endpoint_with_the_directory_type() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("GET", "/directories")
            .match_query(Matcher::UrlEncoded(
                "directory_type".to_string(),
                "gsuite directory".to_string(),
            ))
            .match_header("Authorization", "Bearer sk_example_123456789")
            .with_status(200)
            .with_body(
                json!({
                    "data": [{
                        "id": "directory_01ECAZ4NV9QMV47GW873HDCX74",
                        "domain": "foo-corp.com",
                        "name": "Foo Corp",
                        "organization_id": "org_01EHZNVPK3SFK441A1RGBFSHRT",
                        "object": "directory",
                        "state": "unlinked",
                        "type": "gsuite directory",
                        "created_at": "2021-06-25T19:07:33.155Z",
                        "updated_at": "2021-06-25T19:08:33.155Z"
                        },
                        ],
                        "list_metadata" : {
                        "after" : "directory_01E1JJS84MFPPQ3G655FHTKX6Z",
                        "before" : "directory_01E1JJS84MFPPQ3G655FHTKX6Z"
                        }
                })
                .to_string(),
            )
            .create();

        let paginated_list = workos
            .directory_sync()
            .list_directories(&ListDirectoriesOptions {
                r#type: &Some(KnownOrUnknown::Known(DirectoryType::GoogleWorkspace)),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(
            paginated_list
                .data
                .into_iter()
                .next()
                .map(|directory| directory.id),
            Some(DirectoryId::from("directory_01ECAZ4NV9QMV47GW873HDCX74"))
        )
    }
}
