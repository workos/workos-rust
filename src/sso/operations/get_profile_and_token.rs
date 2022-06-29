use async_trait::async_trait;
use reqwest::StatusCode;
use serde::Deserialize;
use thiserror::Error;

use crate::sso::{AccessToken, AuthorizationCode, ClientId, Profile, Sso};
use crate::{WorkOsError, WorkOsResult};

/// The parameters for [`GetProfileAndToken`].
#[derive(Debug)]
pub struct GetProfileAndTokenParams<'a> {
    /// The client ID corresponding to the environment that SSO was initiated
    /// from.
    pub client_id: &'a ClientId,

    /// The authorization code to exchange for the profile and token.
    pub code: &'a AuthorizationCode,
}

/// The response for [`GetProfileAndToken`].
#[derive(Debug, Deserialize)]
pub struct GetProfileAndTokenResponse {
    /// An access token that can be exchanged for the user profile.
    pub access_token: AccessToken,

    /// The user profile.
    pub profile: Profile,
}

/// An error returned from [`GetProfileAndToken`].
#[derive(Debug, Error, Deserialize)]
#[error("{error}: {error_description}")]
pub struct GetProfileAndTokenError {
    /// The error code of the error that occurred.
    pub error: String,

    /// The description of the error.
    pub error_description: String,
}

/// [WorkOS Docs: Get a Profile and Token](https://workos.com/docs/reference/sso/profile/token)
#[async_trait]
pub trait GetProfileAndToken {
    /// [WorkOS Docs: Get a Profile and Token](https://workos.com/docs/reference/sso/profile/token)
    async fn get_profile_and_token(
        &self,
        params: &GetProfileAndTokenParams<'_>,
    ) -> WorkOsResult<GetProfileAndTokenResponse, GetProfileAndTokenError>;
}

#[async_trait]
impl<'a> GetProfileAndToken for Sso<'a> {
    async fn get_profile_and_token(
        &self,
        params: &GetProfileAndTokenParams<'_>,
    ) -> WorkOsResult<GetProfileAndTokenResponse, GetProfileAndTokenError> {
        let &GetProfileAndTokenParams { client_id, code } = params;

        let url = self.workos.base_url().join("/sso/token")?;
        let params = [
            ("client_id", &client_id.to_string()),
            ("client_secret", &self.workos.key().to_string()),
            ("grant_type", &"authorization_code".to_string()),
            ("code", &code.to_string()),
        ];
        let response = self.workos.client().post(url).form(&params).send().await?;

        match response.error_for_status_ref() {
            Ok(_) => {
                let get_profile_and_token_response =
                    response.json::<GetProfileAndTokenResponse>().await?;

                Ok(get_profile_and_token_response)
            }
            Err(err) => match err.status() {
                Some(StatusCode::BAD_REQUEST) => {
                    let error = response.json::<GetProfileAndTokenError>().await?;

                    Err(match error.error.as_str() {
                        "invalid_client" | "unauthorized_client" => WorkOsError::Unauthorized,
                        _ => WorkOsError::Operation(error),
                    })
                }
                _ => Err(WorkOsError::RequestError(err)),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use matches::assert_matches;
    use mockito::{self, mock, Matcher};
    use serde_json::json;
    use tokio;

    use crate::sso::ProfileId;
    use crate::{ApiKey, WorkOs, WorkOsError};

    use super::*;

    #[tokio::test]
    async fn it_calls_the_token_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("POST", "/sso/token")
            .match_body(Matcher::AllOf(vec![
                Matcher::UrlEncoded("client_id".to_string(), "client_1234".to_string()),
                Matcher::UrlEncoded(
                    "client_secret".to_string(),
                    "sk_example_123456789".to_string(),
                ),
                Matcher::UrlEncoded("grant_type".to_string(), "authorization_code".to_string()),
                Matcher::UrlEncoded("code".to_string(), "abc123".to_string()),
            ]))
            .with_status(200)
            .with_body(
                json!({
                  "access_token": "01DMEK0J53CVMC32CK5SE0KZ8Q",
                  "profile": {
                    "id": "prof_01DMC79VCBZ0NY2099737PSVF1",
                    "connection_id": "conn_01E4ZCR3C56J083X43JQXF3JK5",
                    "connection_type": "okta",
                    "email": "todd@foo-corp.com",
                    "first_name": "Todd",
                    "idp_id": "00u1a0ufowBJlzPlk357",
                    "last_name": "Rundgren",
                    "object": "profile",
                    "raw_attributes": {}
                  }
                })
                .to_string(),
            )
            .create();

        let response = workos
            .sso()
            .get_profile_and_token(&GetProfileAndTokenParams {
                client_id: &ClientId::from("client_1234"),
                code: &AuthorizationCode::from("abc123"),
            })
            .await
            .unwrap();

        assert_eq!(
            response.access_token,
            AccessToken::from("01DMEK0J53CVMC32CK5SE0KZ8Q")
        );
        assert_eq!(
            response.profile.id,
            ProfileId::from("prof_01DMC79VCBZ0NY2099737PSVF1")
        )
    }

    #[tokio::test]
    async fn it_returns_an_unauthorized_error_with_an_invalid_client() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("POST", "/sso/token")
            .with_status(400)
            .with_body(
                json!({
                    "error": "invalid_client",
                    "error_description": "Invalid client ID."
                })
                .to_string(),
            )
            .create();

        let result = workos
            .sso()
            .get_profile_and_token(&GetProfileAndTokenParams {
                client_id: &ClientId::from("client_1234"),
                code: &AuthorizationCode::from("abc123"),
            })
            .await;

        assert_matches!(result, Err(WorkOsError::Unauthorized))
    }

    #[tokio::test]
    async fn it_returns_an_unauthorized_error_with_an_unauthorized_client() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("POST", "/sso/token")
            .with_status(400)
            .with_body(
                json!({
                    "error": "unauthorized_client",
                    "error_description": "Unauthorized"
                })
                .to_string(),
            )
            .create();

        let result = workos
            .sso()
            .get_profile_and_token(&GetProfileAndTokenParams {
                client_id: &ClientId::from("client_1234"),
                code: &AuthorizationCode::from("abc123"),
            })
            .await;

        assert_matches!(result, Err(WorkOsError::Unauthorized))
    }

    #[tokio::test]
    async fn it_returns_an_error_when_the_authorization_code_is_invalid() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("POST", "/sso/token")
            .with_status(400)
            .with_body(
                json!({
                    "error": "invalid_grant",
                    "error_description": "The code 'abc123' has expired or is invalid."
                })
                .to_string(),
            )
            .create();

        let result = workos
            .sso()
            .get_profile_and_token(&GetProfileAndTokenParams {
                client_id: &ClientId::from("client_1234"),
                code: &AuthorizationCode::from("abc123"),
            })
            .await;

        if let Err(WorkOsError::Operation(error)) = result {
            assert_eq!(error.error, "invalid_grant");
            assert_eq!(
                error.error_description,
                "The code 'abc123' has expired or is invalid."
            );
        } else {
            panic!("expected get_profile_and_token to return an error")
        }
    }
}
