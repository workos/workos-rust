use serde::Deserialize;

use crate::directory_sync::DirectoryGroup;

/// [WorkOS Docs: `dsync.group.deleted` Webhook](https://workos.com/docs/reference/webhooks/directory-group#webhooks-dsync.group.deleted)
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DirectoryGroupDeletedWebhook(pub DirectoryGroup);

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use serde_json::{json, Value};

    use crate::directory_sync::{DirectoryGroupId, DirectoryId};

    use crate::webhooks::{Webhook, WebhookEvent, WebhookId};
    use crate::{RawAttributes, Timestamp, Timestamps};

    use super::*;

    #[test]
    fn it_deserializes_a_directory_group_deleted_webhook() {
        let webhook: Webhook = serde_json::from_str(
            &json!(
            {
                "id": "wh_01G69ACR20V4GN4EN7268EFJQ6",
                "event": "dsync.group.deleted",
                "data": {
                  "object": "directory_group",
                  "id": "directory_group_01E1X5GPMMXF4T1DCERMVEEPVW",
                  "directory_id": "directory_01E1X194NTJ3PYMAY79DYV0F0P",
                  "idp_id": "02grqrue4294w24",
                  "name": "Developers",
                  "created_at": "2021-06-25T19:07:33.155Z",
                  "updated_at": "2021-06-25T19:07:33.155Z",
                  "raw_attributes": {
                    "id": "02grqrue4294w24",
                  }
                }
              })
            .to_string(),
        )
        .unwrap();

        let mut expected_raw_attributes = HashMap::new();
        expected_raw_attributes.insert(
            "id".to_string(),
            Value::String("02grqrue4294w24".to_string()),
        );

        assert_eq!(
            webhook,
            Webhook {
                id: WebhookId::from("wh_01G69ACR20V4GN4EN7268EFJQ6"),
                event: WebhookEvent::DirectoryGroupDeleted(DirectoryGroupDeletedWebhook(
                    DirectoryGroup {
                        id: DirectoryGroupId::from("directory_group_01E1X5GPMMXF4T1DCERMVEEPVW"),
                        idp_id: "02grqrue4294w24".to_string(),
                        directory_id: DirectoryId::from("directory_01E1X194NTJ3PYMAY79DYV0F0P"),
                        name: "Developers".to_string(),
                        timestamps: Timestamps {
                            created_at: Timestamp::try_from("2021-06-25T19:07:33.155Z").unwrap(),
                            updated_at: Timestamp::try_from("2021-06-25T19:07:33.155Z").unwrap(),
                        },
                        raw_attributes: RawAttributes(expected_raw_attributes)
                    }
                ))
            }
        )
    }
}
