use async_trait::async_trait;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mfa::{AuthenticationFactor, Mfa};
use crate::{WorkOsError, WorkOsResult};

/// The options for [`EnrollFactor`].
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EnrollFactorOptions<'a> {
    /// Enroll a time-based one-time password (TOTP) factor.
    Totp {
        /// The identifier for the user for whom the factor is being enrolled.
        ///
        /// This is used by authenticator apps to label connections.
        #[serde(rename = "totp_user")]
        user: &'a str,

        /// The identifier for the organization issuing the challenge.
        ///
        /// This should be the name of your application or company.
        #[serde(rename = "totp_issuer")]
        issuer: &'a str,
    },
    /// Enroll an SMS factor.
    Sms {
        /// The phone number for an SMS-enabled device that will receive MFA codes.
        phone_number: &'a str,
    },
}

/// An error returned from [`EnrollFactor`].
#[derive(Debug, Error)]
pub enum EnrollFactorError {
    /// The provided phone number was invalid.
    ///
    /// This can only occur when enrolling an SMS factor.
    #[error("invalid phone number")]
    InvalidPhoneNumber,
}

impl From<EnrollFactorError> for WorkOsError<EnrollFactorError> {
    fn from(err: EnrollFactorError) -> Self {
        Self::Operation(err)
    }
}

#[derive(Debug, Deserialize)]
struct WorkOsApiError {
    pub code: String,
    pub message: String,
}

/// [WorkOS Docs: Enroll Factor](https://workos.com/docs/reference/mfa/enroll-factor)
#[async_trait]
pub trait EnrollFactor {
    /// Enrolls an [`AuthenticationFactor`] to be used as an additional factor of authentication.
    ///
    /// [WorkOS Docs: Enroll Factor](https://workos.com/docs/reference/mfa/enroll-factor)
    async fn enroll_factor(
        &self,
        options: &EnrollFactorOptions<'_>,
    ) -> WorkOsResult<AuthenticationFactor, EnrollFactorError>;
}

#[async_trait]
impl<'a> EnrollFactor for Mfa<'a> {
    async fn enroll_factor(
        &self,
        options: &EnrollFactorOptions<'_>,
    ) -> WorkOsResult<AuthenticationFactor, EnrollFactorError> {
        let url = self.workos.base_url().join("/auth/factors/enroll")?;
        let response = self
            .workos
            .client()
            .post(url)
            .bearer_auth(self.workos.key())
            .json(&options)
            .send()
            .await?;

        match response.error_for_status_ref() {
            Ok(_) => {
                let factor = response.json::<AuthenticationFactor>().await?;

                Ok(factor)
            }
            Err(err) => match err.status() {
                Some(StatusCode::UNPROCESSABLE_ENTITY) => {
                    let error = response.json::<WorkOsApiError>().await?;

                    Err(match error.code.as_str() {
                        "invalid_phone_number" => {
                            WorkOsError::Operation(EnrollFactorError::InvalidPhoneNumber)
                        }
                        _ => WorkOsError::RequestError(err),
                    })
                }
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

    use crate::{mfa::AuthenticationFactorId, ApiKey, WorkOs};

    use super::*;

    #[tokio::test]
    async fn it_calls_the_enroll_factor_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("POST", "/auth/factors/enroll")
            .match_header("Authorization", "Bearer sk_example_123456789")
            .match_body(r#"{"type":"totp","totp_user":"alan.turing@foo-corp.com","totp_issuer":"Foo Corp"}"#)
            .with_status(201)
            .with_body(
                json!({
                    "object": "authentication_factor",
                    "id": "auth_factor_01FVYZ5QM8N98T9ME5BCB2BBMJ",
                    "created_at": "2022-02-15T15:14:19.392Z",
                    "updated_at": "2022-02-15T15:14:19.392Z",
                    "type": "totp",
                    "totp": {
                        "qr_code": "data:image/png;base64,{base64EncodedPng}",
                        "secret": "NAGCCFS3EYRB422HNAKAKY3XDUORMSRF",
                        "uri": "otpauth://totp/FooCorp:alan.turing@foo-corp.com?secret=NAGCCFS3EYRB422HNAKAKY3XDUORMSRF&issuer=FooCorp"
                    }
                  })
                .to_string(),
            )
            .create();

        let factor = workos
            .mfa()
            .enroll_factor(&EnrollFactorOptions::Totp {
                user: "alan.turing@foo-corp.com",
                issuer: "Foo Corp",
            })
            .await
            .unwrap();

        assert_eq!(
            factor.id,
            AuthenticationFactorId::from("auth_factor_01FVYZ5QM8N98T9ME5BCB2BBMJ")
        )
    }

    #[tokio::test]
    async fn it_returns_an_error_when_the_phone_number_is_invalid() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("POST", "/auth/factors/enroll")
            .match_header("Authorization", "Bearer sk_example_123456789")
            .match_body(r#"{"type":"sms","phone_number":"73"}"#)
            .with_status(422)
            .with_body(
                json!({
                    "message": "Phone number is invalid: '73'",
                    "code": "invalid_phone_number"
                })
                .to_string(),
            )
            .create();

        let result = workos
            .mfa()
            .enroll_factor(&EnrollFactorOptions::Sms { phone_number: "73" })
            .await;

        assert_matches!(
            result,
            Err(WorkOsError::Operation(
                EnrollFactorError::InvalidPhoneNumber
            ))
        )
    }
}
