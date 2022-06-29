use async_trait::async_trait;
use reqwest::StatusCode;
use serde::Serialize;

use crate::passwordless::{Passwordless, PasswordlessSessionId};
use crate::{WorkOsError, WorkOsResult};

/// The options for [`SendPasswordlessSession`].
#[derive(Debug, Serialize)]
pub struct SendPasswordlessSessionOptions<'a> {
    /// The ID of the passwordless session to send an email for.
    pub id: &'a PasswordlessSessionId,
}

/// An error returned from [`SendPasswordlessSession`].
#[derive(Debug)]
pub enum SendPasswordlessSessionError {}

/// [WorkOS Docs: Send a Passwordless Session](https://workos.com/docs/reference/magic-link/passwordless-session/send-email)
#[async_trait]
pub trait SendPasswordlessSession {
    /// Sends a [`PasswordlessSession`](crate::passwordless::PasswordlessSession).
    ///
    /// [WorkOS Docs: Send a Passwordless Session](https://workos.com/docs/reference/magic-link/passwordless-session/send-email)
    async fn send_passwordless_session(
        &self,
        options: &SendPasswordlessSessionOptions<'_>,
    ) -> WorkOsResult<(), SendPasswordlessSessionError>;
}

#[async_trait]
impl<'a> SendPasswordlessSession for Passwordless<'a> {
    async fn send_passwordless_session(
        &self,
        options: &SendPasswordlessSessionOptions<'_>,
    ) -> WorkOsResult<(), SendPasswordlessSessionError> {
        let url = self.workos.base_url().join(&format!(
            "/passwordless/sessions/{id}/send",
            id = options.id
        ))?;
        let response = self
            .workos
            .client()
            .post(url)
            .bearer_auth(self.workos.key())
            .json(&options)
            .send()
            .await?;

        match response.error_for_status_ref() {
            Ok(_) => Ok(()),
            Err(err) => match err.status() {
                Some(StatusCode::UNAUTHORIZED) => Err(WorkOsError::Unauthorized),
                _ => Err(WorkOsError::RequestError(err)),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use matches::assert_matches;
    use mockito::{self, mock};
    use serde_json::json;
    use tokio;

    use crate::passwordless::PasswordlessSessionId;
    use crate::{ApiKey, WorkOs};

    use super::*;

    #[tokio::test]
    async fn it_calls_the_send_passwordless_session_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock(
            "POST",
            "/passwordless/sessions/passwordless_session_01EG1BHJMVYMFBQYZTTC0N73CR/send",
        )
        .match_header("Authorization", "Bearer sk_example_123456789")
        .with_status(201)
        .with_body(json!({ "success": true}).to_string())
        .create();

        let result = workos
            .passwordless()
            .send_passwordless_session(&SendPasswordlessSessionOptions {
                id: &PasswordlessSessionId::from("passwordless_session_01EG1BHJMVYMFBQYZTTC0N73CR"),
            })
            .await;

        assert_matches!(result, Ok(()))
    }
}
