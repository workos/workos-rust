use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::webhooks::WebhookEvent;

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
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Webhook {
    /// The ID of the webhook.
    pub id: WebhookId,

    /// The webhook event.
    #[serde(flatten)]
    pub event: WebhookEvent,
}
