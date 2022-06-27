use async_trait::async_trait;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mfa::{AuthenticationFactor, Mfa};
use crate::passwordless::PasswordlessSession;
use crate::{WorkOsError, WorkOsResult};

/// The options for [`CreatePasswordlessSession`].
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CreatePasswordlessSessionOptions<'a> {
    /// Create a Magic Link session.
    #[serde(rename = "MagicLink")]
    MagicLink {
        /// The email of the user to send a Magic Link to.
        email: &'a str,
    },
}

/// An error returned from [`CreatePasswordlessSession`].
#[derive(Debug)]
pub enum CreatePasswordlessSessionError {}

/// [WorkOS Docs: Create a Passwordless Session](https://workos.com/docs/reference/magic-link/passwordless-session/create-session)
#[async_trait]
pub trait CreatePasswordlessSession {
    /// Creates a [`PasswordlessSession`].
    ///
    /// [WorkOS Docs: Create a Passwordless Session](https://workos.com/docs/reference/magic-link/passwordless-session/create-session)
    async fn create_organization(
        &self,
        options: &CreatePasswordlessSessionOptions<'_>,
    ) -> WorkOsResult<PasswordlessSession, CreatePasswordlessSessionError>;
}
