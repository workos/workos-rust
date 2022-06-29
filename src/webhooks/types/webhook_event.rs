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

    /// [WorkOS Docs: `dsync.activated` Webhook](https://workos.com/docs/reference/webhooks/directory#webhooks-dsync.activated)
    #[serde(rename = "dsync.activated")]
    DirectoryActivated(DirectoryActivatedWebhook),

    /// [WorkOS Docs: `dsync.activated` Webhook](https://workos.com/docs/reference/webhooks/directory#webhooks-dsync.deactivated)
    #[serde(rename = "dsync.activated")]
    DirectoryDeactivated(DirectoryDeactivatedWebhook),

    /// [WorkOS Docs: `dsync.activated` Webhook](https://workos.com/docs/reference/webhooks/directory#webhooks-dsync.deleted)
    #[serde(rename = "dsync.activated")]
    DirectoryDeleted(DirectoryDeletedWebhook),

    /// [WorkOS Docs: `connection.deleted` Webhook](https://workos.com/docs/reference/webhooks/connection#webhooks-sso.connection.deleted)
    #[serde(rename = "connection.deleted")]
    ConnectionDeleted(ConnectionDeletedWebhook),

    /// [WorkOS Docs: `dsync.user.created` Webhook](https://workos.com/docs/reference/webhooks/directory-user#webhooks-dsync.user.created)
    #[serde(rename = "dsync.user.created")]
    DirectoryUserCreated(DirectoryUserCreatedWebhook),

    /// [WorkOS Docs: `dsync.user.deleted` Webhook](https://workos.com/docs/reference/webhooks/directory-user#webhooks-dsync.user.deleted)
    #[serde(rename = "dsync.user.deleted")]
    DirectoryUserDeleted(DirectoryUserDeletedWebhook),
}
