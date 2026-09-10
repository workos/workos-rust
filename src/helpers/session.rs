// @oagen-ignore-file
//! Sealed sessions (H04, H05, H06, H07).
//!
//! - Seal/unseal: AES-256-GCM with a 32-byte key derived from a 64-char hex
//!   password. Nonce is prepended to ciphertext, then base64-encoded.
//! - Authenticate: verifies the access token's RS256 signature against the
//!   client's environment JWKS, then validates expiration and identity binding.
//!   Offline verification does not detect revocation before token expiration.
//! - Refresh: trades the refresh token at `/user_management/authenticate`
//!   and re-seals the new session.

use std::time::{SystemTime, UNIX_EPOCH};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD as B64_STANDARD, URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::error::Error;
use crate::models::{AuthenticateResponseImpersonator, User};
use crate::secret::SecretString;

/// Unsealed session cookie payload.
///
/// Tokens are wrapped in [`SecretString`] so they don't leak through the
/// default `Debug` representation. Use [`SecretString::expose`] when the raw
/// value is required (e.g. parsing JWT claims out of the access token).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub access_token: SecretString,
    pub refresh_token: SecretString,
    /// Cookie profile metadata. Authentication binds only `id` to the signed
    /// JWT subject; other fields (including email) are not authorization-grade.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub user: Option<User>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub impersonator: Option<AuthenticateResponseImpersonator>,
}

/// Result of [`SessionManager::authenticate`].
#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub authenticated: bool,
    pub session_id: String,
    pub organization_id: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub entitlements: Vec<String>,
    /// Only `id` is bound to the verified JWT subject. Other profile fields
    /// are cookie metadata and must not be used for authorization decisions.
    pub user: Option<User>,
    /// Impersonation information from the verified JWT, never the cookie alone.
    pub impersonator: Option<AuthenticateResponseImpersonator>,
    /// `true` when the session structure is valid but the access token has
    /// expired. Callers should refresh before treating the user as
    /// unauthenticated.
    pub needs_refresh: bool,
    /// Populated on failure: e.g. `no_session_cookie_provided`,
    /// `invalid_session_cookie`, `invalid_jwt`, `session_expired`.
    pub reason: String,
}

/// Result of [`SessionManager::refresh`].
#[derive(Debug, Clone)]
pub struct SessionRefreshResult {
    pub authenticated: bool,
    pub sealed_session: String,
    pub session: Option<SessionData>,
    pub reason: String,
}

/// Optional knobs for [`SessionManager::refresh`].
#[derive(Debug, Clone, Default)]
pub struct SessionRefreshOptions {
    /// Override the organization id sent on the refresh request. By default
    /// it is extracted from the existing access-token claims.
    pub organization_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct JwtClaims {
    #[serde(default)]
    sub: String,
    #[serde(default)]
    impersonator: Option<AuthenticateResponseImpersonator>,
    #[serde(default)]
    sid: String,
    #[serde(default)]
    org_id: String,
    #[serde(default)]
    role: String,
    #[serde(default)]
    permissions: Vec<String>,
    #[serde(default)]
    entitlements: Vec<String>,
    exp: Option<i64>,
}

/// Wrapper for an existing sealed session cookie.
pub struct SessionManager<'a> {
    client: Option<&'a Client>,
    sealed: String,
    password: String,
}

impl<'a> SessionManager<'a> {
    pub fn new(
        client: Option<&'a Client>,
        sealed: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            client,
            sealed: sealed.into(),
            password: password.into(),
        }
    }

    /// Validates the session cookie. Never errors: failure modes are reported
    /// in the `reason` field of the returned [`SessionState`]. A client with a
    /// configured client ID is required for JWKS verification. Signature/key
    /// failures return `invalid_jwt`; no unverified claims are returned.
    /// This is offline verification, not a check for server-side revocation.
    pub async fn authenticate(&self) -> SessionState {
        if self.sealed.is_empty() {
            return SessionState {
                reason: "no_session_cookie_provided".to_string(),
                ..Default::default()
            };
        }
        let session = match unseal_session(&self.sealed, &self.password) {
            Ok(s) => s,
            Err(_) => {
                return SessionState {
                    reason: "invalid_session_cookie".to_string(),
                    ..Default::default()
                };
            }
        };
        if session.access_token.expose().is_empty() {
            return SessionState {
                reason: "invalid_jwt".to_string(),
                ..Default::default()
            };
        }
        let claims = match self
            .verify_access_token(session.access_token.expose())
            .await
        {
            Ok(c) => c,
            Err(_) => {
                return SessionState {
                    reason: "invalid_jwt".to_string(),
                    ..Default::default()
                };
            }
        };

        if session
            .user
            .as_ref()
            .is_some_and(|user| user.id != claims.sub)
            || session.impersonator.as_ref().is_some_and(|cookie| {
                claims.impersonator.as_ref().is_none_or(|signed| {
                    cookie.email != signed.email || cookie.reason != signed.reason
                })
            })
        {
            return SessionState {
                reason: "invalid_jwt".to_string(),
                ..Default::default()
            };
        }

        let Some(exp) = claims.exp else {
            return SessionState {
                reason: "invalid_jwt".to_string(),
                ..Default::default()
            };
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        if now >= exp {
            return SessionState {
                authenticated: false,
                needs_refresh: true,
                session_id: claims.sid,
                organization_id: claims.org_id,
                role: claims.role,
                permissions: claims.permissions,
                entitlements: claims.entitlements,
                user: session.user,
                impersonator: claims.impersonator,
                reason: "session_expired".to_string(),
            };
        }

        SessionState {
            authenticated: true,
            session_id: claims.sid,
            organization_id: claims.org_id,
            role: claims.role,
            permissions: claims.permissions,
            entitlements: claims.entitlements,
            user: session.user,
            impersonator: claims.impersonator,
            ..Default::default()
        }
    }

    async fn verify_access_token(&self, token: &str) -> Result<JwtClaims, Error> {
        let invalid = || Error::Jwt("access token verification failed".to_string());
        let client = self
            .client
            .filter(|c| !c.client_id().is_empty())
            .ok_or_else(invalid)?;
        let header = decode_header(token).map_err(|_| invalid())?;
        if header.alg != Algorithm::RS256 {
            return Err(invalid());
        }
        let kid = header
            .kid
            .filter(|kid| !kid.is_empty())
            .ok_or_else(invalid)?;
        let jwks = client.session_jwks();
        let mut set = jwks.fetch().await?;
        if !set.keys.iter().any(|key| key.kid.as_deref() == Some(&kid)) {
            // A new signing key may have appeared since the cached fetch.
            set = jwks.refresh_if_unchanged(Some(&set)).await?;
        }
        let mut matching = set
            .keys
            .iter()
            .filter(|key| key.kid.as_deref() == Some(&kid));
        let key = matching.next().ok_or_else(invalid)?;
        if matching.next().is_some()
            || key.kty.as_deref() != Some("RSA")
            || key.alg.as_deref().is_some_and(|alg| alg != "RS256")
            || key.use_.as_deref().is_some_and(|usage| usage != "sig")
        {
            return Err(invalid());
        }
        if let Some(ops) = key.other.get("key_ops") {
            let ops = ops.as_array().ok_or_else(invalid)?;
            if !ops.iter().any(|op| op.as_str() == Some("verify")) {
                return Err(invalid());
            }
        }
        let jwk = serde_json::from_value(serde_json::to_value(key)?).map_err(|_| invalid())?;
        let decoding_key = DecodingKey::from_jwk(&jwk).map_err(|_| invalid())?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_required_spec_claims(&["exp", "sub"]);
        // Expiration is checked only after signature verification so a genuinely
        // expired token can signal refresh. Preserve strict, zero-leeway expiry.
        validation.validate_exp = false;
        validation.leeway = 0;
        validation.validate_nbf = true;
        // WorkOS environment selection is via the client-specific JWKS, not aud.
        validation.validate_aud = false;
        let claims = decode::<JwtClaims>(token, &decoding_key, &validation)
            .map_err(|_| invalid())?
            .claims;
        if claims.sub.is_empty() {
            return Err(invalid());
        }
        Ok(claims)
    }

    /// Refreshes the session by exchanging the refresh token. Re-seals the
    /// new session and returns it.
    pub async fn refresh(
        &self,
        opts: SessionRefreshOptions,
    ) -> Result<SessionRefreshResult, Error> {
        if self.sealed.is_empty() {
            return Ok(SessionRefreshResult {
                authenticated: false,
                sealed_session: String::new(),
                session: None,
                reason: "no_session_cookie_provided".to_string(),
            });
        }
        let session = match unseal_session(&self.sealed, &self.password) {
            Ok(s) => s,
            Err(_) => {
                return Ok(SessionRefreshResult {
                    authenticated: false,
                    sealed_session: String::new(),
                    session: None,
                    reason: "invalid_session_cookie".to_string(),
                });
            }
        };
        if session.refresh_token.expose().is_empty() {
            return Ok(SessionRefreshResult {
                authenticated: false,
                sealed_session: String::new(),
                session: None,
                reason: "no_refresh_token".to_string(),
            });
        }
        let client = self
            .client
            .ok_or_else(|| Error::Builder("client is required for session refresh".to_string()))?;

        let org_id = opts.organization_id.or_else(|| {
            parse_jwt_payload(session.access_token.expose())
                .ok()
                .and_then(|c| (!c.org_id.is_empty()).then_some(c.org_id))
        });

        let body = serde_json::json!({
            "grant_type": "refresh_token",
            "client_id": client.client_id(),
            "client_secret": client.api_key(),
            "refresh_token": session.refresh_token,
            "organization_id": org_id,
        });

        let auth: crate::models::AuthenticateResponse = match client
            .request_json(http::Method::POST, "/user_management/authenticate", &body)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let reason = match e.api() {
                    Some(api)
                        if api.status == 401 && api.code.as_deref() == Some("invalid_grant") =>
                    {
                        "refresh_token_revoked"
                    }
                    _ => "refresh_failed",
                };
                return Ok(SessionRefreshResult {
                    authenticated: false,
                    sealed_session: String::new(),
                    session: None,
                    reason: reason.to_string(),
                });
            }
        };

        let new_session = SessionData {
            access_token: auth.access_token,
            refresh_token: auth.refresh_token,
            user: Some(auth.user),
            impersonator: auth.impersonator,
        };
        let sealed = seal_session(&new_session, &self.password)?;
        Ok(SessionRefreshResult {
            authenticated: true,
            sealed_session: sealed,
            session: Some(new_session),
            reason: String::new(),
        })
    }

    /// Builds a logout redirect URL using the session's claimed `sid`.
    pub async fn logout_url(&self, return_to: Option<&str>) -> Result<String, Error> {
        let state = self.authenticate().await;
        if (!state.authenticated && !state.needs_refresh) || state.session_id.is_empty() {
            return Err(Error::Session(
                "session is not authenticated or has no session ID".to_string(),
            ));
        }
        let base = self
            .client
            .map(|c| c.base_url().to_string())
            .unwrap_or_else(|| crate::client::DEFAULT_BASE_URL.to_string());
        let mut url = url::Url::parse(&format!("{base}/user_management/sessions/logout"))
            .map_err(|e| Error::Builder(format!("invalid base URL: {e}")))?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("session_id", &state.session_id);
            if let Some(rt) = return_to
                && !rt.is_empty()
            {
                q.append_pair("return_to", rt);
            }
        }
        Ok(url.into())
    }
}

/// Convenience: unseal + authenticate without constructing a manager.
pub async fn authenticate_session(client: &Client, sealed: &str, password: &str) -> SessionState {
    client.session(sealed, password).authenticate().await
}

/// Seal an arbitrary serializable value.
pub fn seal<T: Serialize>(data: &T, password: &str) -> Result<String, Error> {
    let plaintext = serde_json::to_vec(data).map_err(Error::from)?;
    seal_bytes(&plaintext, password)
}

/// Unseal into an arbitrary deserializable value.
pub fn unseal<T: for<'de> Deserialize<'de>>(sealed: &str, password: &str) -> Result<T, Error> {
    let bytes = unseal_bytes(sealed, password)?;
    serde_json::from_slice(&bytes).map_err(Error::from)
}

/// Seal a [`SessionData`] for use as a session cookie.
pub fn seal_session(data: &SessionData, password: &str) -> Result<String, Error> {
    seal(data, password)
}

/// Unseal into a [`SessionData`].
pub fn unseal_session(sealed: &str, password: &str) -> Result<SessionData, Error> {
    unseal(sealed, password)
}

/// Build a sealed session from raw token + user fields.
pub fn seal_session_from_auth_response(
    access_token: impl Into<SecretString>,
    refresh_token: impl Into<SecretString>,
    user: Option<User>,
    impersonator: Option<AuthenticateResponseImpersonator>,
    password: &str,
) -> Result<String, Error> {
    let session = SessionData {
        access_token: access_token.into(),
        refresh_token: refresh_token.into(),
        user,
        impersonator,
    };
    seal_session(&session, password)
}

fn seal_bytes(plaintext: &[u8], password: &str) -> Result<String, Error> {
    let key = derive_key(password)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| Error::Session(format!("init AES-GCM: {e}")))?;
    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| Error::Session(format!("encrypt: {e}")))?;
    let mut buf = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
    buf.extend_from_slice(&nonce_bytes);
    buf.extend_from_slice(&ciphertext);
    Ok(B64_STANDARD.encode(buf))
}

fn unseal_bytes(sealed: &str, password: &str) -> Result<Vec<u8>, Error> {
    let raw = B64_STANDARD
        .decode(sealed)
        .map_err(|e| Error::Session(format!("base64 decode: {e}")))?;
    if raw.len() < 12 {
        return Err(Error::Session("sealed data too short".to_string()));
    }
    let key = derive_key(password)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| Error::Session(format!("init AES-GCM: {e}")))?;
    let (nonce_bytes, ciphertext) = raw.split_at(12);
    cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|e| Error::Session(format!("decrypt: {e}")))
}

fn derive_key(password: &str) -> Result<[u8; 32], Error> {
    if password.len() != 64 {
        return Err(Error::Session(format!(
            "cookie password must be a 64-character hex string (32 bytes); got length {}",
            password.len()
        )));
    }
    let bytes = hex::decode(password)
        .map_err(|e| Error::Session(format!("cookie password must be valid hex: {e}")))?;
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

// Unverified organization hint for refresh only. The WorkOS refresh endpoint
// authorizes the exchange; never use these claims to authenticate a session.
fn parse_jwt_payload(token: &str) -> Result<JwtClaims, Error> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(Error::Jwt(format!(
            "invalid JWT format: expected 3 parts, got {}",
            parts.len()
        )));
    }
    let decoded = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| Error::Jwt(format!("decode payload: {e}")))?;
    serde_json::from_slice(&decoded).map_err(|e| Error::Jwt(format!("parse claims: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pwd() -> &'static str {
        // 64 hex chars = 32 bytes.
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    }

    #[test]
    fn seal_and_unseal_value() {
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
        struct Foo {
            a: i32,
            b: String,
        }
        let v = Foo {
            a: 7,
            b: "hi".into(),
        };
        let sealed = seal(&v, pwd()).unwrap();
        let back: Foo = unseal(&sealed, pwd()).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn unseal_rejects_wrong_password() {
        let sealed = seal(&"hi", pwd()).unwrap();
        let bad = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        assert!(unseal::<String>(&sealed, bad).is_err());
    }

    #[test]
    fn derive_key_rejects_short_password() {
        assert!(derive_key("short").is_err());
    }

    #[tokio::test]
    async fn authenticate_no_cookie() {
        let s = SessionManager::new(None, "", pwd()).authenticate().await;
        assert!(!s.authenticated);
        assert_eq!(s.reason, "no_session_cookie_provided");
    }

    #[tokio::test]
    async fn authenticate_invalid_cookie() {
        let s = SessionManager::new(None, "not-base64@@@", pwd())
            .authenticate()
            .await;
        assert!(!s.authenticated);
        assert_eq!(s.reason, "invalid_session_cookie");
    }

    #[test]
    fn parse_jwt_payload_basic() {
        // header.payload.sig — payload encodes {"sid":"sess_1","exp":9999999999}.
        let payload = URL_SAFE_NO_PAD.encode(br#"{"sid":"sess_1","exp":9999999999}"#);
        let token = format!("h.{payload}.s");
        let claims = parse_jwt_payload(&token).unwrap();
        assert_eq!(claims.sid, "sess_1");
        assert_eq!(claims.exp, Some(9_999_999_999));
    }

    #[tokio::test]
    async fn authenticate_without_client_fails_closed() {
        let payload =
            URL_SAFE_NO_PAD.encode(br#"{"sub":"user_1","sid":"sess_1","exp":9999999999}"#);
        let session = SessionData {
            access_token: format!("h.{payload}.s").into(),
            refresh_token: "r".into(),
            user: None,
            impersonator: None,
        };
        let sealed = seal_session(&session, pwd()).unwrap();
        let state = SessionManager::new(None, sealed, pwd())
            .authenticate()
            .await;
        assert!(!state.authenticated);
        assert!(!state.needs_refresh);
        assert_eq!(state.reason, "invalid_jwt");
    }
}
