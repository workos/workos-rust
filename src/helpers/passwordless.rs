// @oagen-ignore-file
//! Passwordless (magic-link) sessions — Cat 2.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Error;
use crate::helpers::util::percent_encode;

/// A magic-link passwordless session.
#[derive(Debug, Clone, Deserialize)]
pub struct PasswordlessSession {
    pub id: String,
    pub email: String,
    pub expires_at: String,
    pub link: String,
    pub object: String,
}

/// Type of passwordless session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PasswordlessSessionType {
    MagicLink,
}

impl PasswordlessSessionType {
    fn as_str(self) -> &'static str {
        match self {
            PasswordlessSessionType::MagicLink => "MagicLink",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PasswordlessCreateSessionParams {
    pub email: String,
    #[serde(rename = "type", serialize_with = "serialize_session_type")]
    pub kind: PasswordlessSessionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<i64>,
}

fn serialize_session_type<S>(
    value: &PasswordlessSessionType,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(value.as_str())
}

/// Passwordless API handle. Obtain via [`crate::Client::passwordless`].
pub struct PasswordlessApi<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> PasswordlessApi<'a> {
    /// Create a passwordless session — `POST /passwordless/sessions`.
    pub async fn create_session(
        &self,
        params: PasswordlessCreateSessionParams,
    ) -> Result<PasswordlessSession, Error> {
        self.client
            .request_json(http::Method::POST, "/passwordless/sessions", &params)
            .await
    }

    /// Send the magic-link email — `POST /passwordless/sessions/{id}/send`.
    pub async fn send_session(&self, session_id: &str) -> Result<(), Error> {
        let path = format!("/passwordless/sessions/{}/send", percent_encode(session_id));
        self.client
            .request_empty::<()>(http::Method::POST, &path, None)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_params_serialize_with_renamed_type() {
        let p = PasswordlessCreateSessionParams {
            email: "a@b.com".to_string(),
            kind: PasswordlessSessionType::MagicLink,
            redirect_uri: None,
            state: None,
            expires_in: None,
        };
        let s = serde_json::to_string(&p).unwrap();
        assert!(s.contains(r#""type":"MagicLink""#));
        assert!(s.contains(r#""email":"a@b.com""#));
        // Optional fields omitted.
        assert!(!s.contains("redirect_uri"));
    }
}
