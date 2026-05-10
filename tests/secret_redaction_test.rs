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
