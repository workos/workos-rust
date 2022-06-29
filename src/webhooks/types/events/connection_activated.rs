use serde::Deserialize;

use crate::sso::Connection;

#[derive(Debug, Deserialize)]
pub struct ConnectionActivatedWebhook(pub Connection);
