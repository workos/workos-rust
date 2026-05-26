// @oagen-ignore-file
//! Hand-maintained helper layer.
//!
//! These modules implement non-spec endpoints (Passwordless, Vault) and
//! client-side helpers (webhook signature verification, PKCE, sealed sessions,
//! JWKS, AuthKit/SSO URL builders, etc.) that oagen does not generate.

pub mod actions;
pub mod authkit;
pub mod jwks;
pub mod passwordless;
pub mod pkce;
pub mod public_client;
pub mod session;
pub mod sso_helpers;
mod util;
pub mod vault_crypto;
pub mod webhook_verification;

pub use actions::{ActionSignedResponse, ActionType, ActionVerdict, ActionsHelper};
pub use authkit::{
    AuthKitAuthorizationUrlParams, AuthKitHelper, AuthKitPkceAuthorizationUrl,
    AuthKitPkceCodeExchangeParams,
};
pub use jwks::{Jwk, JwkSet, JwksHelper, jwks_url};
pub use passwordless::{
    PasswordlessApi, PasswordlessCreateSessionParams, PasswordlessSession, PasswordlessSessionType,
};
pub use pkce::{PkcePair, generate_code_challenge, generate_code_verifier, generate_pkce_pair};
pub use public_client::PublicClient;
pub use session::{
    SessionData, SessionManager, SessionRefreshOptions, SessionRefreshResult, SessionState, seal,
    seal_session, unseal, unseal_session,
};
pub use sso_helpers::{
    SsoAuthorizationUrlParams, SsoHelper, SsoLogoutUrlParams, SsoPkceAuthorizationUrl,
    SsoPkceCodeExchangeParams,
};
pub use vault_crypto::{VaultEncryptResult, extract_encrypted_keys, local_decrypt, local_encrypt};
pub use webhook_verification::{
    WebhookVerifier, compute_webhook_signature, parse_webhook_signature_header,
};
