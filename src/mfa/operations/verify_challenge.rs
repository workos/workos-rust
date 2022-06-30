use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mfa::{AuthenticationChallenge, AuthenticationChallengeId, Mfa, MfaCode};
use crate::{ResponseExt, WorkOsResult};

/// The response for [`VerifyChallenge`].
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyChallengeResponse {
    /// The challenge that was verified.
    pub challenge: AuthenticationChallenge,

    /// Whether the challenge was verified successfully.
    #[serde(rename = "valid")]
    pub is_valid: bool,
}

/// The parameters for [`VerifyChallenge`].
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct VerifyChallengeParams<'a> {
    /// The ID of the authentication challenge to verify.
    pub authentication_challenge_id: &'a AuthenticationChallengeId,

    /// The MFA code to verify.
    pub code: &'a MfaCode,
}

/// An error returned from [`VerifyChallenge`].
#[derive(Debug, Error)]
pub enum VerifyChallengeError {}

/// [WorkOS Docs: Verify Challenge](https://workos.com/docs/reference/mfa/verify-factor)
#[async_trait]
pub trait VerifyChallenge {
    /// Attempts a verification for an authentication challenge.
    ///
    /// [WorkOS Docs: Verify Challenge](https://workos.com/docs/reference/mfa/verify-factor)
    async fn verify_challenge(
        &self,
        params: &VerifyChallengeParams<'_>,
    ) -> WorkOsResult<VerifyChallengeResponse, VerifyChallengeError>;
}

#[async_trait]
impl<'a> VerifyChallenge for Mfa<'a> {
    async fn verify_challenge(
        &self,
        params: &VerifyChallengeParams<'_>,
    ) -> WorkOsResult<VerifyChallengeResponse, VerifyChallengeError> {
        let url = self.workos.base_url().join("/auth/factors/verify")?;
        let verify_response = self
            .workos
            .client()
            .post(url)
            .bearer_auth(self.workos.key())
            .json(&params)
            .send()
            .await?
            .handle_unauthorized_or_generic_error()?
            .json::<VerifyChallengeResponse>()
            .await?;

        Ok(verify_response)
    }
}

#[cfg(test)]
mod test {
    use mockito::{self, mock};
    use serde_json::json;
    use tokio;

    use crate::{
        mfa::{AuthenticationChallengeId, MfaCode},
        ApiKey, WorkOs,
    };

    use super::*;

    #[tokio::test]
    async fn it_calls_the_verify_challenge_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("POST", "/auth/factors/verify")
            .match_header("Authorization", "Bearer sk_example_123456789")
            .match_body(
                r#"{"authentication_challenge_id":"auth_challenge_01FVYZWQTZQ5VB6BC5MPG2EYC5","code":"123456"}"#,
            )
            .with_status(201)
            .with_body(
                json!({
                  "challenge": {
                    "object": "authentication_challenge",
                    "id": "auth_challenge_01FVYZWQTZQ5VB6BC5MPG2EYC5",
                    "created_at": "2022-02-15T15:26:53.274Z",
                    "updated_at": "2022-02-15T15:26:53.274Z",
                    "expires_at": "2022-02-15T15:36:53.279Z",
                    "authentication_factor_id": "auth_factor_01FVYZ5QM8N98T9ME5BCB2BBMJ"
                  },
                  "valid": true
                })
                .to_string(),
            )
            .create();

        let verify = workos
            .mfa()
            .verify_challenge(&VerifyChallengeParams {
                authentication_challenge_id: &AuthenticationChallengeId::from(
                    "auth_challenge_01FVYZWQTZQ5VB6BC5MPG2EYC5",
                ),
                code: &MfaCode::from("123456"),
            })
            .await
            .unwrap();

        assert_eq!(
            verify.challenge.id,
            AuthenticationChallengeId::from("auth_challenge_01FVYZWQTZQ5VB6BC5MPG2EYC5")
        )
    }
}
