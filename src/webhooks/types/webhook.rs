use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::webhooks::{WebhookEvent, WebhookEventDto};

/// The ID of a [`Webhook`].
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WebhookId(String);

impl Display for WebhookId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for WebhookId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for WebhookId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug)]
pub struct Webhook {
    pub id: WebhookId,
    pub event: WebhookEvent,
}

impl Webhook {
    pub fn from_str(payload: &str) -> serde_json::Result<Self> {
        #[derive(Debug, Deserialize)]
        struct WebhookDto {
            pub id: WebhookId,
        }

        let webhook: WebhookDto = serde_json::from_str(payload)?;
        let event: WebhookEventDto = serde_json::from_str(payload)?;

        Ok(Self {
            id: webhook.id,
            event: event.into(),
        })
    }
}

#[cfg(test)]
mod test {
    use matches::assert_matches;
    use serde_json::json;

    use super::*;

    #[test]
    fn it_deserializes_a_connection_activated_webhook() {
        let webhook = Webhook::from_str(
            &json!({
              "id": "wh_01G699XH8F3MAJJWSHZFQ3WWVX",
              "event": "connection.activated",
              "data": {
                "object": "connection",
                "id": "conn_01EHWNC0FCBHZ3BJ7EGKYXK0E6",
                "organization_id": "org_01EHWNCE74X7JSDV0X3SZ3KJNY",
                "external_key": "3QMR4u0Tok6SgwY2AWG6u6mkQ",
                "connection_type": "OktaSAML",
                "name": "Foo Corp's Connection",
                "state": "active",
                "status": "linked",
                "domains": [
                  {
                    "object": "connection_domain",
                    "id": "conn_domain_01EHWNFTAFCF3CQAE5A9Q0P1YB",
                    "domain": "foo-corp.com"
                  }
                ],
                "created_at": "2021-06-25T19:07:33.155Z",
                "updated_at": "2021-06-25T19:07:33.155Z"
              }
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(webhook.id, WebhookId::from("wh_01G699XH8F3MAJJWSHZFQ3WWVX"));

        assert_matches!(webhook.event, WebhookEvent::ConnectionActivated(_))
    }
}
