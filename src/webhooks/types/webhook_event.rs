use serde::Deserialize;

use super::events::*;

#[derive(Debug)]
pub enum WebhookEvent {
    ConnectionActivated(ConnectionActivatedWebhook),
}

impl From<WebhookEventDto> for WebhookEvent {
    fn from(event: WebhookEventDto) -> Self {
        match event {
            WebhookEventDto::ConnectionActivated(payload) => Self::ConnectionActivated(payload),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "event", content = "data")]
pub(crate) enum WebhookEventDto {
    #[serde(rename = "connection.activated")]
    ConnectionActivated(ConnectionActivatedWebhook),
}
