use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mfa::{AuthenticationChallenge, AuthenticationChallengeId, Mfa, MfaCode};
use crate::{ResponseExt, WorkOsResult};

/// Payload returned from [`VerifyFactor`] including Authentication Factor and Valid status.
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyFactorResponse {
    /// The Authentication Challenge being verified against.
    pub challenge: AuthenticationChallenge,

    /// A valid boolean value to indicate if the code was correct.
    #[serde(rename = "valid")]
    pub is_valid: bool,
}

/// The parameters for [`VerifyFactor`].
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct VerifyFactorParams<'a> {
    /// The ID of the authentication factor to verify.
    pub authentication_challenge_id: &'a AuthenticationChallengeId,

    /// The 6 digit code to be verified.
    pub code: &'a MfaCode,
}

/// An error returned from [`VerifyFactor`].
#[derive(Debug, Error)]
pub enum VerifyFactorError {}

/// [WorkOS Docs: Verify Factor](https://workos.com/docs/reference/mfa/verify-factor)
#[async_trait]
pub trait VerifyFactor {
    /// Attempts a verification for an authentication challenge.
    ///
    /// [WorkOS Docs: Verify Factor](https://workos.com/docs/reference/mfa/verify-factor)
    async fn verify_factor(
        &self,
        params: &VerifyFactorParams<'_>,
    ) -> WorkOsResult<VerifyFactorResponse, VerifyFactorError>;
}

#[async_trait]
impl<'a> VerifyFactor for Mfa<'a> {
    async fn verify_factor(
        &self,
        params: &VerifyFactorParams<'_>,
    ) -> WorkOsResult<VerifyFactorResponse, VerifyFactorError> {
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
            .json::<VerifyFactorResponse>()
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
    async fn it_calls_the_verify_factor_endpoint() {
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
            .verify_factor(&VerifyFactorParams {
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
