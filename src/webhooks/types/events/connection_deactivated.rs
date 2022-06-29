use serde::Deserialize;

use crate::sso::Connection;

/// [WorkOS Docs: `connection.deactivated` Webhook](https://workos.com/docs/reference/webhooks/connection#webhooks-sso.connection.deactivated)
#[derive(Debug, Deserialize)]
pub struct ConnectionDeactivatedWebhook(pub Connection);
