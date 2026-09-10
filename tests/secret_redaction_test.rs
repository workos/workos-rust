// @oagen-ignore-file
//! Verifies the SDK's secret-field redaction contract: structs whose fields
//! the emitter classified as sensitive must not surface the underlying value
//! through their default `Debug` representation.

use workos::SecretString;

#[test]
fn secret_string_debug_redacts() {
    let s = SecretString::new("super-secret-token");
    let d = format!("{s:?}");
    assert!(!d.contains("super-secret-token"));
    assert_eq!(d, "SecretString(\"<redacted>\")");
}

#[test]
fn session_data_debug_redacts_tokens() {
    let session = workos::SessionData {
        access_token: "leaked-access-token".into(),
        refresh_token: "leaked-refresh-token".into(),
        user: None,
        impersonator: None,
    };
    let d = format!("{session:?}");
    assert!(
        !d.contains("leaked-access-token"),
        "access_token leaked in Debug: {d}"
    );
    assert!(
        !d.contains("leaked-refresh-token"),
        "refresh_token leaked in Debug: {d}"
    );
}

#[test]
fn generated_password_request_debug_redacts() {
    // `AuthenticateWithPasswordParams` exposes `password: SecretString` because
    // `password` matches the sensitive-field heuristic in the Rust emitter.
    let params = workos::user_management::AuthenticateWithPasswordParams::new(
        "user@example.com",
        "should-never-appear-in-debug",
    );
    let d = format!("{params:?}");
    assert!(
        !d.contains("should-never-appear-in-debug"),
        "password leaked in Debug: {d}"
    );
}

// VULN-1265: generic names and encoded/token-bearing siblings must be redacted,
// not just fields whose names already match the emitter's heuristic.
macro_rules! model_debug_redacts {
    ($test:ident, $model:ty, $fixture:literal, [$($field:literal),+]) => {
        #[test]
        fn $test() -> Result<(), serde_json::Error> {
            let fixture: serde_json::Value =
                serde_json::from_str(include_str!($fixture))?;
            let mut leaked_fields = Vec::new();
            // Check each field independently so an already-redacted sibling
            // cannot hide a missing wrapper on a URI, QR code, or URL.
            for field in [$($field),+] {
                let sentinel = format!("vuln-1265-{field}-must-not-appear");
                let mut input = fixture.clone();
                input[field] = serde_json::Value::String(sentinel.clone());
                let model: $model = serde_json::from_value(input)?;
                if format!("{model:?}").contains(&sentinel) {
                    leaked_fields.push(field);
                }
            }
            assert!(leaked_fields.is_empty(), "Debug leaked fields: {leaked_fields:?}");
            Ok(())
        }
    };
}

model_debug_redacts!(
    data_integration_credentials_response_credential_debug_redacts,
    workos::DataIntegrationCredentialsResponseCredential,
    "fixtures/data_integration_credentials_response_credential.json",
    ["value"]
);

model_debug_redacts!(
    authentication_factor_enrolled_totp_debug_redacts,
    workos::AuthenticationFactorEnrolledTotp,
    "fixtures/authentication_factor_enrolled_totp.json",
    ["secret", "qr_code", "uri"]
);

model_debug_redacts!(
    magic_auth_debug_redacts,
    workos::MagicAuth,
    "fixtures/magic_auth.json",
    ["code"]
);

model_debug_redacts!(
    magic_auth_send_magic_auth_code_and_return_response_debug_redacts,
    workos::MagicAuthSendMagicAuthCodeAndReturnResponse,
    "fixtures/magic_auth_send_magic_auth_code_and_return_response.json",
    ["code"]
);

model_debug_redacts!(
    create_object_request_debug_redacts,
    workos::CreateObjectRequest,
    "fixtures/create_object_request.json",
    ["value"]
);

model_debug_redacts!(
    update_object_request_debug_redacts,
    workos::UpdateObjectRequest,
    "fixtures/update_object_request.json",
    ["value"]
);

model_debug_redacts!(
    vault_object_debug_redacts,
    workos::VaultObject,
    "fixtures/vault_object.json",
    ["value"]
);

model_debug_redacts!(
    authenticate_response_debug_redacts,
    workos::AuthenticateResponse,
    "fixtures/authenticate_response.json",
    ["authkit_authorization_code"]
);

model_debug_redacts!(
    user_api_key_with_value_debug_redacts,
    workos::UserApiKeyWithValue,
    "fixtures/user_api_key_with_value.json",
    ["value"]
);

model_debug_redacts!(
    organization_api_key_with_value_debug_redacts,
    workos::OrganizationApiKeyWithValue,
    "fixtures/organization_api_key_with_value.json",
    ["value"]
);

model_debug_redacts!(
    validate_api_key_debug_redacts,
    workos::ValidateApiKey,
    "fixtures/validate_api_key.json",
    ["value"]
);

model_debug_redacts!(
    password_reset_debug_redacts,
    workos::PasswordReset,
    "fixtures/password_reset.json",
    ["password_reset_token", "password_reset_url"]
);

model_debug_redacts!(
    user_invite_debug_redacts,
    workos::UserInvite,
    "fixtures/user_invite.json",
    ["token", "accept_invitation_url"]
);

model_debug_redacts!(
    device_authorization_response_debug_redacts,
    workos::DeviceAuthorizationResponse,
    "fixtures/device_authorization_response.json",
    ["device_code", "user_code", "verification_uri_complete"]
);

model_debug_redacts!(
    device_code_session_authenticate_request_debug_redacts,
    workos::DeviceCodeSessionAuthenticateRequest,
    "fixtures/device_code_session_authenticate_request.json",
    ["device_code"]
);

model_debug_redacts!(
    email_verification_debug_redacts,
    workos::EmailVerification,
    "fixtures/email_verification.json",
    ["code"]
);

model_debug_redacts!(
    authentication_challenge_debug_redacts,
    workos::AuthenticationChallenge,
    "fixtures/authentication_challenge.json",
    ["code"]
);

model_debug_redacts!(
    portal_link_response_debug_redacts,
    workos::PortalLinkResponse,
    "fixtures/portal_link_response.json",
    ["link"]
);

model_debug_redacts!(
    sso_logout_authorize_response_debug_redacts,
    workos::SSOLogoutAuthorizeResponse,
    "fixtures/sso_logout_authorize_response.json",
    ["logout_token", "logout_url"]
);

// Additional same-class fields found by reconciling the model sweep and
// reviewing credential/code names (including the PKCE proof key).
model_debug_redacts!(
    agent_admin_validate_credential_request_debug_redacts,
    workos::AgentAdminValidateCredentialRequest,
    "fixtures/agent_admin_validate_credential_request.json",
    ["credential"]
);

model_debug_redacts!(
    authentication_challenges_verify_request_debug_redacts,
    workos::AuthenticationChallengesVerifyRequest,
    "fixtures/authentication_challenges_verify_request.json",
    ["code"]
);

model_debug_redacts!(
    authorization_code_session_authenticate_request_debug_redacts,
    workos::AuthorizationCodeSessionAuthenticateRequest,
    "fixtures/authorization_code_session_authenticate_request.json",
    ["code", "code_verifier"]
);

model_debug_redacts!(
    claim_view_response_debug_redacts,
    workos::ClaimViewResponse,
    "fixtures/claim_view_response.json",
    ["user_code"]
);

model_debug_redacts!(
    confirm_email_change_debug_redacts,
    workos::ConfirmEmailChange,
    "fixtures/confirm_email_change.json",
    ["code"]
);

model_debug_redacts!(
    create_connection_key_pair_debug_redacts,
    workos::CreateConnectionKeyPair,
    "fixtures/create_connection_key_pair.json",
    ["key"]
);

model_debug_redacts!(
    email_verification_code_session_authenticate_request_debug_redacts,
    workos::EmailVerificationCodeSessionAuthenticateRequest,
    "fixtures/email_verification_code_session_authenticate_request.json",
    ["code"]
);

model_debug_redacts!(
    invitation_debug_redacts,
    workos::Invitation,
    "fixtures/invitation.json",
    ["accept_invitation_url"]
);

model_debug_redacts!(
    magic_auth_code_session_authenticate_request_debug_redacts,
    workos::MagicAuthCodeSessionAuthenticateRequest,
    "fixtures/magic_auth_code_session_authenticate_request.json",
    ["code"]
);

model_debug_redacts!(
    mfa_totp_session_authenticate_request_debug_redacts,
    workos::MfaTotpSessionAuthenticateRequest,
    "fixtures/mfa_totp_session_authenticate_request.json",
    ["code"]
);

model_debug_redacts!(
    radar_challenge_debug_redacts,
    workos::RadarChallenge,
    "fixtures/radar_challenge.json",
    ["code"]
);

model_debug_redacts!(
    radar_email_challenge_code_session_authenticate_request_debug_redacts,
    workos::RadarEmailChallengeCodeSessionAuthenticateRequest,
    "fixtures/radar_email_challenge_code_session_authenticate_request.json",
    ["code"]
);

model_debug_redacts!(
    radar_sms_challenge_code_session_authenticate_request_debug_redacts,
    workos::RadarSmsChallengeCodeSessionAuthenticateRequest,
    "fixtures/radar_sms_challenge_code_session_authenticate_request.json",
    ["code"]
);

model_debug_redacts!(
    token_query_debug_redacts,
    workos::TokenQuery,
    "fixtures/token_query.json",
    ["code"]
);

model_debug_redacts!(
    verify_email_address_debug_redacts,
    workos::VerifyEmailAddress,
    "fixtures/verify_email_address.json",
    ["code"]
);

#[test]
fn authentication_wrapper_debug_redacts_code_and_verifier() {
    let mut params = workos::user_management::AuthenticateWithCodeParams::new("secret-auth-code");
    params.code_verifier = Some("secret-pkce-verifier".into());
    let debug = format!("{params:?}");
    assert!(!debug.contains("secret-auth-code"));
    assert!(!debug.contains("secret-pkce-verifier"));
}

#[test]
fn device_verification_uri_redacts_only_code_bearing_form() -> Result<(), serde_json::Error> {
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/device_authorization_response.json"))?;
    let public_uri = "https://auth.example.com/device";
    let complete_uri = format!("{public_uri}?user_code=secret-device-user-code");
    input["verification_uri"] = serde_json::json!(public_uri);
    input["verification_uri_complete"] = serde_json::json!(complete_uri);
    input["user_code"] = serde_json::json!("secret-device-user-code");
    let response: workos::DeviceAuthorizationResponse = serde_json::from_value(input.clone())?;
    let debug = format!("{response:?}");
    assert!(debug.contains(public_uri));
    assert!(!debug.contains("secret-device-user-code"));
    let output = serde_json::to_value(response)?;
    for field in ["verification_uri", "verification_uri_complete", "user_code"] {
        assert_eq!(output[field], input[field]);
    }
    Ok(())
}

#[test]
fn public_certificate_remains_visible() -> Result<(), serde_json::Error> {
    let pair: workos::CreateConnectionKeyPair = serde_json::from_value(serde_json::json!({
        "key": "secret-private-key",
        "cert": "public-x509-certificate"
    }))?;
    let debug = format!("{pair:?}");
    assert!(!debug.contains("secret-private-key"));
    assert!(debug.contains("public-x509-certificate"));
    Ok(())
}

fn assert_wire_round_trip<T>(fixture: &str) -> Result<(), serde_json::Error>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let mut input: serde_json::Value = serde_json::from_str(fixture)?;
    input["value"] = serde_json::json!("vuln-1265-wire-secret");
    // Compare canonical JSON bytes: fixture whitespace/key ordering is not
    // meaningful, but the fields and their plain-string values must not change.
    let expected = serde_json::to_vec(&input)?;
    let model: T = serde_json::from_slice(&expected)?;
    let output = serde_json::to_value(&model)?;
    assert_eq!(serde_json::to_vec(&output)?, expected);
    let reparsed: T = serde_json::from_value(output)?;
    assert_eq!(serde_json::to_value(reparsed)?, input);
    Ok(())
}

#[test]
fn vault_object_secret_wire_round_trip() -> Result<(), serde_json::Error> {
    assert_wire_round_trip::<workos::VaultObject>(include_str!("fixtures/vault_object.json"))
}

#[test]
fn user_api_key_with_value_secret_wire_round_trip() -> Result<(), serde_json::Error> {
    assert_wire_round_trip::<workos::UserApiKeyWithValue>(include_str!(
        "fixtures/user_api_key_with_value.json"
    ))
}
