use serde::Deserialize;

use super::events::*;

/// The event of a [`Webhook`](crate::webhooks::Webhook).
#[derive(Debug)]
pub enum WebhookEvent {
    /// [WorkOS Docs: `connection.activated` Webhook](https://workos.com/docs/reference/webhooks/connection#webhooks-sso.connection.activated)
    ConnectionActivated(ConnectionActivatedWebhook),
}

impl From<WebhookEventDto> for WebhookEvent {
    fn from(event: WebhookEventDto) -> Self {
        match event {
            WebhookEventDto::ConnectionActivated(payload) => Self::ConnectionActivated(payload),
        }
    }
}

/// A DTO for deserializing a webhook event from JSON.
#[derive(Debug, Deserialize)]
#[serde(tag = "event", content = "data")]
pub(crate) enum WebhookEventDto {
    #[serde(rename = "connection.activated")]
    ConnectionActivated(ConnectionActivatedWebhook),
}
