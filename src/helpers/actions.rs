// @oagen-ignore-file
//! AuthKit Actions request verification + response signing (H03).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64_STANDARD;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

use crate::error::Error;
use crate::helpers::webhook_verification::{
    compute_webhook_signature, parse_webhook_signature_header,
};
use crate::models::{Invitation, Organization, OrganizationMembership, User};

const DEFAULT_TOLERANCE: Duration = Duration::from_secs(30);

/// Type of an AuthKit Action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    #[serde(rename = "authentication")]
    Authentication,
    #[serde(rename = "user_registration")]
    UserRegistration,
}

impl ActionType {
    fn as_str(self) -> &'static str {
        match self {
            ActionType::Authentication => "authentication",
            ActionType::UserRegistration => "user_registration",
        }
    }
}

/// Verdict returned in an action response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionVerdict {
    Allow,
    Deny,
}

impl ActionVerdict {
    fn as_str(self) -> &'static str {
        match self {
            ActionVerdict::Allow => "Allow",
            ActionVerdict::Deny => "Deny",
        }
    }
}

/// The provisional user data carried by a `user_registration` action context.
#[derive(Debug, Clone, Deserialize)]
pub struct ActionUserData {
    /// Discriminator `"user_data"`.
    pub object: String,
    /// The email address the user is registering with.
    pub email: String,
    /// The user's full name, or `None`/null.
    pub name: Option<String>,
    /// The user's first name.
    pub first_name: String,
    /// The user's last name.
    pub last_name: String,
}

/// A verified, deserialized AuthKit Action request.
///
/// WorkOS sends a flat context object discriminated by `object`, not the
/// webhook event envelope:
/// - `authentication_action_context`: `user`, `organization`,
///   `organization_membership`, `issuer`
/// - `user_registration_action_context`: `user_data`, `invitation`
///
/// `ip_address`, `user_agent`, and `device_fingerprint` are shared by both
/// context types; fields specific to the other variant are `None`.
#[derive(Debug, Clone, Deserialize)]
pub struct ActionContext {
    /// Discriminates the action context type.
    pub object: String,
    /// Unique identifier for the action context.
    pub id: String,
    /// Caller IP address (both context types).
    pub ip_address: Option<String>,
    /// Caller user agent (both context types).
    pub user_agent: Option<String>,
    /// Caller device fingerprint (both context types).
    pub device_fingerprint: Option<String>,
    /// Present when `object == "authentication_action_context"`.
    pub user: Option<User>,
    /// Present when `object == "authentication_action_context"`.
    pub organization: Option<Organization>,
    /// Present when `object == "authentication_action_context"`.
    pub organization_membership: Option<OrganizationMembership>,
    /// Present when `object == "authentication_action_context"`.
    pub issuer: Option<String>,
    /// Present when `object == "user_registration_action_context"`.
    pub user_data: Option<ActionUserData>,
    /// Present when `object == "user_registration_action_context"`.
    pub invitation: Option<Invitation>,
}

/// Result of signing an action response. Send `payload` and `sig` back to WorkOS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSignedResponse {
    pub payload: String,
    pub sig: String,
}

/// Helpers for AuthKit Actions: request verification + response signing.
pub struct ActionsHelper {
    tolerance: Duration,
    now: Box<dyn Fn() -> SystemTime + Send + Sync>,
}

impl Default for ActionsHelper {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionsHelper {
    pub fn new() -> Self {
        Self {
            tolerance: DEFAULT_TOLERANCE,
            now: Box::new(SystemTime::now),
        }
    }

    pub fn with_tolerance(mut self, tolerance: Duration) -> Self {
        self.tolerance = tolerance;
        self
    }

    pub fn with_clock(mut self, now: impl Fn() -> SystemTime + Send + Sync + 'static) -> Self {
        self.now = Box::new(now);
        self
    }

    /// Verifies the signature header against `payload`.
    pub fn verify_header(
        &self,
        payload: &str,
        sig_header: &str,
        secret: &str,
    ) -> Result<(), Error> {
        if sig_header.is_empty() {
            return Err(Error::Webhook("webhook not signed".to_string()));
        }
        let (timestamp, signature) = parse_webhook_signature_header(sig_header)?;
        let ts: u64 = timestamp
            .parse()
            .map_err(|_| Error::Webhook("invalid timestamp in signature header".to_string()))?;
        let signed_at = UNIX_EPOCH
            .checked_add(Duration::from_millis(ts))
            .ok_or_else(|| Error::Webhook("invalid timestamp in signature header".to_string()))?;
        let now = (self.now)();
        let diff = match now.duration_since(signed_at) {
            Ok(d) => d,
            Err(e) => e.duration(),
        };
        if diff > self.tolerance {
            return Err(Error::Webhook("timestamp outside tolerance".to_string()));
        }
        let expected = compute_webhook_signature(secret, &timestamp, payload);
        if expected.as_bytes().ct_eq(signature.as_bytes()).unwrap_u8() != 1 {
            return Err(Error::Webhook("no valid signature found".to_string()));
        }
        Ok(())
    }

    /// Verifies and deserializes the action payload into an `ActionContext`.
    /// Dispatch on `object` to read the type-specific fields.
    pub fn construct_action(
        &self,
        payload: &str,
        sig_header: &str,
        secret: &str,
    ) -> Result<ActionContext, Error> {
        self.verify_header(payload, sig_header, secret)?;
        serde_json::from_str(payload).map_err(Error::from)
    }

    /// Signs an action response with `secret`.
    pub fn sign_response(
        &self,
        action_type: ActionType,
        verdict: ActionVerdict,
        error_message: &str,
        secret: &str,
    ) -> Result<ActionSignedResponse, Error> {
        let body = serde_json::json!({
            "type": action_type.as_str(),
            "verdict": verdict.as_str(),
            "error_message": error_message,
        });
        let json_bytes = serde_json::to_vec(&body).map_err(Error::from)?;
        let b64_payload = B64_STANDARD.encode(&json_bytes);

        let now = (self.now)();
        let ts_ms = now
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::Crypto(format!("clock before epoch: {e}")))?
            .as_millis() as i64;
        let timestamp = ts_ms.to_string();
        let sig = compute_webhook_signature(secret, &timestamp, &b64_payload);

        Ok(ActionSignedResponse {
            payload: b64_payload,
            sig: format!("t={timestamp},v1={sig}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_now(ms: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(ms)
    }

    #[test]
    fn verify_header_round_trip() {
        let secret = "shh";
        let payload = "hello";
        let ts = "1700000000000";
        let sig = compute_webhook_signature(secret, ts, payload);
        let header = format!("t={ts},v1={sig}");
        let helper = ActionsHelper::new().with_clock(|| fixed_now(1_700_000_000_000));
        helper.verify_header(payload, &header, secret).unwrap();
    }

    #[test]
    fn verify_header_rejects_outside_tolerance() {
        let secret = "shh";
        let payload = "hello";
        let signed = 1_700_000_000_000u64;
        let sig = compute_webhook_signature(secret, &signed.to_string(), payload);
        let header = format!("t={signed},v1={sig}");
        let helper = ActionsHelper::new().with_clock(move || fixed_now(signed + 60_000));
        assert!(helper.verify_header(payload, &header, secret).is_err());
    }

    #[test]
    fn sign_response_round_trips() {
        let secret = "shh";
        let helper = ActionsHelper::new().with_clock(|| fixed_now(1_700_000_000_000));
        let signed = helper
            .sign_response(ActionType::Authentication, ActionVerdict::Allow, "", secret)
            .unwrap();
        // Decode payload — must be valid base64 JSON with the right shape.
        let bytes = B64_STANDARD.decode(&signed.payload).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["type"], "authentication");
        assert_eq!(parsed["verdict"], "Allow");
        // Sig header is t=...,v1=... with verifiable signature.
        let (ts, sig) = parse_webhook_signature_header(&signed.sig).unwrap();
        let expected = compute_webhook_signature(secret, &ts, &signed.payload);
        assert_eq!(sig, expected);
    }

    fn action_sig_header(secret: &str, payload: &str) -> String {
        let ts = "1700000000000";
        let sig = compute_webhook_signature(secret, ts, payload);
        format!("t={ts},v1={sig}")
    }

    #[test]
    fn construct_action_authentication() {
        let secret = "shh";
        let payload = r#"{"object":"authentication_action_context","id":"action_01","user":{"object":"user","id":"user_01","email":"test@example.com","email_verified":true,"created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z"},"organization":{"object":"organization","id":"org_01","name":"Acme","domains":[],"metadata":{},"created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z"},"ip_address":"1.2.3.4","user_agent":"curl/8","device_fingerprint":"fp_123","issuer":"https://auth.example.com"}"#;
        let header = action_sig_header(secret, payload);
        let helper = ActionsHelper::new().with_clock(|| fixed_now(1_700_000_000_000));
        let action = helper.construct_action(payload, &header, secret).unwrap();
        assert_eq!(action.object, "authentication_action_context");
        assert_eq!(action.id, "action_01");
        assert_eq!(action.ip_address.as_deref(), Some("1.2.3.4"));
        assert_eq!(action.user_agent.as_deref(), Some("curl/8"));
        assert_eq!(action.device_fingerprint.as_deref(), Some("fp_123"));
        assert_eq!(action.issuer.as_deref(), Some("https://auth.example.com"));
        assert_eq!(action.user.as_ref().unwrap().id, "user_01");
        assert_eq!(action.user.as_ref().unwrap().email, "test@example.com");
        assert_eq!(action.organization.as_ref().unwrap().id, "org_01");
        assert!(action.user_data.is_none());
        assert!(action.invitation.is_none());
    }

    #[test]
    fn construct_action_user_registration() {
        let secret = "shh";
        let payload = r#"{"object":"user_registration_action_context","id":"action_02","user_data":{"object":"user_data","email":"new@example.com","first_name":"New","last_name":"User","name":null},"ip_address":"5.6.7.8","user_agent":"Mozilla/5.0","device_fingerprint":"fp_456"}"#;
        let header = action_sig_header(secret, payload);
        let helper = ActionsHelper::new().with_clock(|| fixed_now(1_700_000_000_000));
        let action = helper.construct_action(payload, &header, secret).unwrap();
        assert_eq!(action.object, "user_registration_action_context");
        assert_eq!(action.id, "action_02");
        assert_eq!(action.ip_address.as_deref(), Some("5.6.7.8"));
        let user_data = action.user_data.as_ref().unwrap();
        assert_eq!(user_data.email, "new@example.com");
        assert_eq!(user_data.first_name, "New");
        assert_eq!(user_data.last_name, "User");
        assert!(user_data.name.is_none());
        assert!(action.user.is_none());
        assert!(action.organization.is_none());
        assert!(action.invitation.is_none());
    }
}
