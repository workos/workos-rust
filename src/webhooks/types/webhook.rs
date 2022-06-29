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

/// A WorkOS webhook.
#[derive(Debug, PartialEq, Eq)]
pub struct Webhook {
    /// The ID of the webhook.
    pub id: WebhookId,

    /// The webhook event.
    pub event: WebhookEvent,
}

impl Webhook {
    /// Parses a [`Webhook`] from a JSON string.
    pub fn from_str(payload: &str) -> serde_json::Result<Self> {
        #[derive(Debug, Deserialize)]
        struct WebhookDto {
            pub id: WebhookId,
        }

        // Deserialize the two different parts of the webhook separately, since
        // combining both `struct` and `enum` semantics in the same type is a bit
        // of a headache.
        let webhook: WebhookDto = serde_json::from_str(payload)?;
        let event: WebhookEventDto = serde_json::from_str(payload)?;

        Ok(Self {
            id: webhook.id,
            event: event.into(),
        })
    }
}
