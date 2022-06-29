use serde::Deserialize;

use super::events::*;

/// The event of a [`Webhook`](crate::webhooks::Webhook).
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum WebhookEvent {
    /// [WorkOS Docs: `connection.activated` Webhook](https://workos.com/docs/reference/webhooks/connection#webhooks-sso.connection.activated)
    #[serde(rename = "connection.activated")]
    ConnectionActivated(ConnectionActivatedWebhook),

    /// [WorkOS Docs: `connection.deactivated` Webhook](https://workos.com/docs/reference/webhooks/connection#webhooks-sso.connection.deactivated)
    #[serde(rename = "connection.deactivated")]
    ConnectionDeactivated(ConnectionDeactivatedWebhook),
}
