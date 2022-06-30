use serde::Deserialize;

use crate::directory_sync::{DirectoryGroup, DirectoryId, DirectoryUser};

///Payload Structure for [dsync.group.user_added` Webhook](https://workos.com/docs/reference/webhooks/directory-group#webhooks-dsync.group.user_added)
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct DirectoryUserAddedToGroupWebhook {
    /// [`DirectoryId`] in [dsync.group.user_added` Webhook](https://workos.com/docs/reference/webhooks/directory-group#webhooks-dsync.group.user_added)
    pub directory_id: DirectoryId,

    /// [`DirectoryUser`] (appears as "user") in [dsync.group.user_added` Webhook](https://workos.com/docs/reference/webhooks/directory-group#webhooks-dsync.group.user_added)
    pub user: DirectoryUser,

    /// [`DirectoryGroup`] (appears as "group") in [dsync.group.user_added` Webhook](https://workos.com/docs/reference/webhooks/directory-group#webhooks-dsync.group.user_added)
    pub group: DirectoryGroup,
}

/// [WorkOS Docs: `dsync.group.user_added` Webhook](https://workos.com/docs/reference/webhooks/directory-group#webhooks-dsync.group.user_added)
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct DirectoryGroupUserAddedWebhook(pub DirectoryUserAddedToGroupWebhook);

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use serde_json::{json, Value};

    use crate::directory_sync::{
        DirectoryGroup, DirectoryGroupId, DirectoryId, DirectoryUser, DirectoryUserEmail,
        DirectoryUserId, DirectoryUserState,
    };

    use crate::webhooks::{Webhook, WebhookEvent, WebhookId};
    use crate::{KnownOrUnknown, RawAttributes, Timestamp, Timestamps};

    use super::*;

    #[test]
    fn it_deserializes_a_directory_group_user_added_webhook() {
        let webhook: Webhook = serde_json::from_str(
            &json!({
              "id": "wh_04FKJ843CVE8F7BXQSPFH0M53V",
              "data": {
                "directory_id": "directory_01ECAZ4NV9QMV47GW873HDCX74",
                "user": {
                  "id": "directory_user_01E1X56GH84T3FB41SD6PZGDBX",
                  "idp_id": "2936",
                  "emails": [{
                    "primary": true,
                    "type": "work",
                    "value": "eric@foo-corp.com"
                  }],
                  "first_name": "Eric",
                  "last_name": "Schneider",
                  "username": "eric@foo-corp.com",
                  "state": "active",
                  "created_at": "2021-06-25T19:07:33.155Z",
                  "updated_at": "2021-06-25T19:07:33.155Z",
                  "custom_attributes": {
                    "department": "Engineering"
                  },
                  "raw_attributes": {"idp_id":"1a2b3c4d5e"}
                },
                "group": {
                    "id": "directory_group_01E1JJS84MFPPQ3G655FHTKX6Z",
                    "idp_id": "02grqrue4294w24",
                    "directory_id": "directory_01ECAZ4NV9QMV47GW873HDCX74",
                    "name": "Developers",
                    "created_at": "2021-06-25T19:07:33.155Z",
                    "updated_at": "2021-06-25T19:07:33.155Z",
                    "raw_attributes": {
                      "id": "12345"
                  }}
              },
              "event": "dsync.group.user_added"
            })
            .to_string(),
        )
        .unwrap();

        let mut expected_custom_attributes = HashMap::new();
        expected_custom_attributes.insert(
            "department".to_string(),
            Value::String("Engineering".to_string()),
        );

        let mut expected_user_raw_attributes = HashMap::new();
        expected_user_raw_attributes.insert(
            "idp_id".to_string(),
            Value::String("1a2b3c4d5e".to_string()),
        );

        let mut expected_group_raw_attributes = HashMap::new();
        expected_group_raw_attributes.insert("id".to_string(), Value::String("12345".to_string()));

        assert_eq!(
            webhook,
            Webhook {
                id: WebhookId::from("wh_04FKJ843CVE8F7BXQSPFH0M53V"),
                event: WebhookEvent::DirectoryGroupUserAdded(DirectoryGroupUserAddedWebhook(
                    DirectoryUserAddedToGroupWebhook {
                        directory_id: DirectoryId::from("directory_01ECAZ4NV9QMV47GW873HDCX74"),
                        user: DirectoryUser {
                            id: DirectoryUserId::from("directory_user_01E1X56GH84T3FB41SD6PZGDBX"),
                            state: KnownOrUnknown::Known(DirectoryUserState::Active),
                            timestamps: Timestamps {
                                created_at: Timestamp::try_from("2021-06-25T19:07:33.155Z")
                                    .unwrap(),
                                updated_at: Timestamp::try_from("2021-06-25T19:07:33.155Z")
                                    .unwrap()
                            },
                            idp_id: "12345".to_string(),
                            directory_id: DirectoryId::from("directory_01ECAZ4NV9QMV47GW873HDCX74"),
                            username: Some("eric@foo-corp.com".to_string()),
                            emails: vec![DirectoryUserEmail {
                                primary: Some(true),
                                r#type: Some("work".to_string()),
                                value: Some("eric@foo-corp.com".to_string())
                            }],
                            first_name: Some("Eric".to_string()),
                            last_name: Some("Schneider".to_string()),
                            custom_attributes: expected_custom_attributes,
                            raw_attributes: RawAttributes(expected_user_raw_attributes),
                        },
                        group: DirectoryGroup {
                            id: DirectoryGroupId::from(
                                "directory_group_01E1JJS84MFPPQ3G655FHTKX6Z"
                            ),
                            idp_id: "12345".to_string(),
                            directory_id: DirectoryId::from("directory_01ECAZ4NV9QMV47GW873HDCX74"),
                            name: "Developers".to_string(),
                            timestamps: Timestamps {
                                created_at: Timestamp::try_from("2021-06-25T19:07:33.155Z")
                                    .unwrap(),
                                updated_at: Timestamp::try_from("2021-06-25T19:07:33.155Z")
                                    .unwrap(),
                            },
                            raw_attributes: RawAttributes(expected_group_raw_attributes)
                        }
                    }
                ))
            }
        )
    }
}
