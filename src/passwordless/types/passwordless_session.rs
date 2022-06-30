use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::Timestamp;

/// The ID of an [`PasswordlessSession`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PasswordlessSessionId(String);

impl Display for PasswordlessSessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for PasswordlessSessionId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for PasswordlessSessionId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// The type of a [`PasswordlessSession`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PasswordlessSessionType {
    /// Magic Link.
    MagicLink {
        /// The email address of the user the Magic Link was sent to.
        email: String,

        /// The Magic Link for the user to authenticate with.
        link: String,
    },
}

/// [WorkOS Docs: Passwordless Session](https://workos.com/docs/reference/magic-link/passwordless-session)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordlessSession {
    /// The ID of the passwordless session.
    pub id: PasswordlessSessionId,

    /// The type of the passwordless session.
    #[serde(flatten)]
    pub r#type: PasswordlessSessionType,

    /// The timestamp indicating when the passwordless session will expire.
    pub expires_at: Timestamp,
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::Timestamp;

    use super::{PasswordlessSession, PasswordlessSessionId, PasswordlessSessionType};

    #[test]
    fn it_deserializes_a_magic_link_session() {
        let passwordless_session: PasswordlessSession = serde_json::from_str(
            &json!({
                "object": "passwordless_session",
                "id": "passwordless_session_01EHDAK2BFGWCSZXP9HGZ3VK8C",
                "email": "marcelina@foo-corp.com",
                "expires_at": "2020-08-13T05:50:00.000Z",
                "link": "https://auth.workos.com/passwordless/4TeRexuejWCKs9rrFOIuLRYEr/confirm"
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(
            passwordless_session,
            PasswordlessSession {
                id: PasswordlessSessionId::from("passwordless_session_01EHDAK2BFGWCSZXP9HGZ3VK8C"),
                r#type: PasswordlessSessionType::MagicLink {
                    email: "marcelina@foo-corp.com".to_string(),
                    link: "https://auth.workos.com/passwordless/4TeRexuejWCKs9rrFOIuLRYEr/confirm"
                        .to_string(),
                },
                expires_at: Timestamp::try_from("2020-08-13T05:50:00.000Z").unwrap()
            }
        )
    }
}
