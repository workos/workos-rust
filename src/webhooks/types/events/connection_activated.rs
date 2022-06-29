use serde::Deserialize;

use crate::sso::Connection;

/// [WorkOS Docs: `connection.activated` Webhook](https://workos.com/docs/reference/webhooks/connection#webhooks-sso.connection.activated)
#[derive(Debug, Deserialize)]
pub struct ConnectionActivatedWebhook(pub Connection);
