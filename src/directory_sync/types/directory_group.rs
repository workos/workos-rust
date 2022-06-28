use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::{directory_sync::DirectoryId, RawAttributes, Timestamps};

/// The ID of a [`DirectoryGroup`].
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DirectoryGroupId(String);

impl Display for DirectoryGroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for DirectoryGroupId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for DirectoryGroupId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// [WorkOS Docs: Directory Group](https://workos.com/docs/reference/directory-sync/directory-group)
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryGroup {
    /// Unique identifier for the Directory Group.
    pub id: DirectoryGroupId,

    /// Unique identifier for the group, assigned by the Directory Provider.
    /// Different Directory Providers use different ID formats.
    pub idp_id: String,

    /// The identifier of the [`Directory`] the Directory Group belongs to.
    pub directory_id: DirectoryId,

    /// The name of the Directory Group.
    pub name: String,

    /// The timestamps for the Directory Group.
    #[serde(flatten)]
    pub timestamps: Timestamps,

    /// The raw attributes received from the Identity Provider.
    pub raw_attributes: RawAttributes,
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use serde_json::{json, Value};

    use crate::{RawAttributes, Timestamp, Timestamps};

    use super::{DirectoryGroup, DirectoryGroupId, DirectoryId};

    #[test]
    fn it_deserializes_a_directory_group() {
        let directory_group: DirectoryGroup = serde_json::from_str(
            &json!({
              "id": "directory_group_01E1JJS84MFPPQ3G655FHTKX6Z",
              "idp_id": "02grqrue4294w24",
              "directory_id": "directory_01ECAZ4NV9QMV47GW873HDCX74",
              "name": "Developers",
              "created_at": "2021-06-25T19:07:33.155Z",
              "updated_at": "2021-06-25T19:07:33.155Z",
              "raw_attributes": {
                "idp_id": "12345"
            }})
            .to_string(),
        )
        .unwrap();

        let mut expected_raw_attributes = HashMap::new();
        expected_raw_attributes.insert("idp_id".to_string(), Value::String("12345".to_string()));

        assert_eq!(
            directory_group,
            DirectoryGroup {
                id: DirectoryGroupId::from("directory_group_01E1JJS84MFPPQ3G655FHTKX6Z"),
                idp_id: "02grqrue4294w24".to_string(),
                directory_id: DirectoryId::from("directory_01ECAZ4NV9QMV47GW873HDCX74"),
                name: "Developers".to_string(),
                timestamps: Timestamps {
                    created_at: Timestamp::try_from("2021-06-25T19:07:33.155Z").unwrap(),
                    updated_at: Timestamp::try_from("2021-06-25T19:07:33.155Z").unwrap(),
                },
                raw_attributes: RawAttributes(expected_raw_attributes)
            }
        )
    }
}
