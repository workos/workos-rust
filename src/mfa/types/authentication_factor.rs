use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// The ID of an [`AuthenticationFactor`].
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AuthenticationFactorId(String);

impl Display for AuthenticationFactorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for AuthenticationFactorId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for AuthenticationFactorId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// [WorkOS Docs: Authentication Factor](https://workos.com/docs/reference/mfa/authentication-factor)
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticationFactor {
    /// The ID of the authentication factor.
    pub id: AuthenticationFactorId,

    /// The type of the authentication factor.
    #[serde(flatten)]
    pub r#type: AuthenticationFactorType,
}

/// The type of an [`AuthenticationFactor`].
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationFactorType {
    /// Time-based one-time password (TOTP).
    Totp {
        /// A [data URL](https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/Data_URLs)
        /// containing the scannable QR code to enroll the factor.
        qr_code: String,

        /// The TOTP secret.
        ///
        /// This can be manually entered into some authenticator apps in place of scanning the [`qr_code`].
        secret: String,

        /// The `otpauth://` URI that is encoded in the [`qr_code`].
        uri: String,
    },
    ///
    Sms {
        /// The phone number the factor was enrolled with.
        phone_number: String,
    },
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::{AuthenticationFactor, AuthenticationFactorId, AuthenticationFactorType};

    #[test]
    fn it_deserializes_a_totp_factor() {
        let factor: AuthenticationFactor = serde_json::from_str(&json!({
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
          }).to_string()).unwrap();

        assert_eq!(
            factor,
            AuthenticationFactor {
                id: AuthenticationFactorId::from("auth_factor_01FVYZ5QM8N98T9ME5BCB2BBMJ"),
                r#type: AuthenticationFactorType::Totp {
                    qr_code: "data:image/png;base64,{base64EncodedPng}".to_string(),
                    secret: "NAGCCFS3EYRB422HNAKAKY3XDUORMSRF".to_string(),
                    uri: "otpauth://totp/FooCorp:alan.turing@foo-corp.com?secret=NAGCCFS3EYRB422HNAKAKY3XDUORMSRF&issuer=FooCorp".to_string()
                }
            }
        )
    }

    #[test]
    fn it_deserializes_an_sms_factor() {
        let factor: AuthenticationFactor = serde_json::from_str(
            &json!({
              "object": "authentication_factor",
              "id": "auth_factor_01FVYZ5QM8N98T9ME5BCB2BBMJ",
              "created_at": "2022-02-15T15:14:19.392Z",
              "updated_at": "2022-02-15T15:14:19.392Z",
              "type": "sms",
              "sms": {
                  "phone_number": "+15005550006"
              }
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(
            factor,
            AuthenticationFactor {
                id: AuthenticationFactorId::from("auth_factor_01FVYZ5QM8N98T9ME5BCB2BBMJ"),
                r#type: AuthenticationFactorType::Sms {
                    phone_number: "+15005550006".to_string()
                }
            }
        )
    }
}
