use crate::Timestamps;

use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::directory_sync::DirectoryType;
use crate::organizations::OrganizationId;
use crate::KnownOrUnknown;

/// The ID of a [`Directory`].
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DirectoryId(String);

impl Display for DirectoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for DirectoryId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for DirectoryId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// The state of a [`Directory`].
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryState {
    /// The directory is linked.
    Linked,

    /// The directory is unlinked.
    Unlinked,

    /// The directory is attemping to link.
    Validating,

    /// The directory is being deleted.
    Deleting,

    /// The directory was unable to link due to invalid credentials.
    InvalidCredentials,
}

/// [WorkOS Docs: Directory](https://workos.com/docs/reference/directory-sync/directory)
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Directory {
    /// The ID of the directory.
    pub id: DirectoryId,

    /// The ID of the associated [`Organization`] for this directory.
    pub organization_id: Option<OrganizationId>,

    /// The type of the directory.
    pub r#type: KnownOrUnknown<DirectoryType, String>,

    /// The state of the directory.
    pub state: DirectoryState,

    /// The name of the directory.
    pub name: String,

    /// The URL associated with an Enterprise Client.
    pub domain: Option<String>,

    /// The timestamps for the Directory.
    #[serde(flatten)]
    pub timestamps: Timestamps,
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::directory_sync::DirectoryType;
    use crate::organizations::OrganizationId;
    use crate::KnownOrUnknown;
    use crate::Timestamps;
    use chrono::DateTime;

    use super::{Directory, DirectoryId, DirectoryState};

    #[test]
    fn it_deserializes_a_directory() {
        let directory: Directory = serde_json::from_str(
            &json!({
              "id": "directory_01ECAZ4NV9QMV47GW873HDCX74",
              "domain": "foo-corp.com",
              "name": "Foo Corp",
              "organization_id": "org_01EHZNVPK3SFK441A1RGBFSHRT",
              "state": "unlinked",
              "type": "bamboohr",
              "created_at": "2021-06-25T19:07:33.155Z",
              "updated_at": "2021-06-25T19:07:33.155Z"
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(
            directory,
            Directory {
                id: DirectoryId::from("directory_01ECAZ4NV9QMV47GW873HDCX74"),
                domain: Some("foo-corp.com".to_string()),
                organization_id: Some(OrganizationId::from("org_01EHZNVPK3SFK441A1RGBFSHRT")),
                r#type: KnownOrUnknown::Known(DirectoryType::BambooHr),
                name: "Foo Corp".to_string(),
                state: DirectoryState::Unlinked,
                timestamps: Timestamps {
                    created_at: DateTime::parse_from_rfc3339("2021-06-25T19:07:33.155Z").unwrap(),
                    updated_at: DateTime::parse_from_rfc3339("2021-06-25T19:07:33.155Z").unwrap(),
                }
            }
        )
    }

    #[test]
    fn it_deserializes_unknown_directory_types() {
        let directory: Directory = serde_json::from_str(
            &json!({
              "id": "directory_01ECAZ4NV9QMV47GW873HDCX74",
              "domain": "foo-corp.com",
              "name": "Foo Corp",
              "organization_id": "org_01EHZNVPK3SFK441A1RGBFSHRT",
              "state": "unlinked",
              "type": "UnknownType",
              "created_at": "2021-06-25T19:07:33.155Z",
              "updated_at": "2021-06-25T19:07:33.155Z"
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(
            directory.r#type,
            KnownOrUnknown::Unknown("UnknownType".to_string())
        )
    }
}
