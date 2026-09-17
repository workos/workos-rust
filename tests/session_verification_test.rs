// @oagen-ignore-file
//! Regression coverage for VULN-1238. All signing keys are test-only fixtures.
// Test fixture helpers intentionally panic on invalid setup.
#![allow(clippy::unwrap_used)]

use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};
use workos::helpers::session::authenticate_session;
use workos::{
    Client,
    helpers::{SessionData, SessionState, seal_session},
    models::User,
};

const PASSWORD: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn keys() -> Value {
    serde_json::from_str(include_str!("fixtures/session_jwt_keys.json")).unwrap()
}

fn claims() -> Value {
    json!({"sub":"user_1", "sid":"sess_1", "org_id":"org_1", "role":"admin",
        "permissions":["read"], "entitlements":["enterprise"], "exp":9999999999u64})
}

fn token(claims: &Value, signer: &str, kid: Option<&str>) -> String {
    let private = STANDARD
        .decode(keys()[signer]["private_der"].as_str().unwrap())
        .unwrap();
    let mut header = Header::new(Algorithm::RS256);
    header.kid = kid.map(str::to_string);
    encode(&header, claims, &EncodingKey::from_rsa_der(&private)).unwrap()
}

fn user(id: &str) -> User {
    serde_json::from_value(json!({"object":"user", "id":id, "email":"user@example.com",
        "email_verified":true, "created_at":"", "updated_at":""}))
    .unwrap()
}

fn session(token: String) -> SessionData {
    SessionData {
        access_token: token.into(),
        refresh_token: "refresh".into(),
        user: Some(user("user_1")),
        impersonator: None,
    }
}

async fn setup() -> (MockServer, Client) {
    let server = MockServer::start().await;
    let client = Client::builder()
        .api_key("sk_test")
        .client_id("client_test")
        .base_url(server.uri())
        .build();
    (server, client)
}

async fn serve_keys(server: &MockServer, jwks: Value, requests: u64) {
    Mock::given(method("GET"))
        .and(path("/sso/jwks/client_test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(jwks))
        .expect(requests)
        .mount(server)
        .await;
}

async fn authenticate(client: &Client, session: &SessionData) -> SessionState {
    let sealed = seal_session(session, PASSWORD).unwrap();
    authenticate_session(client, &sealed, PASSWORD).await
}

fn assert_rejected(state: SessionState) {
    assert!(!state.authenticated);
    assert!(!state.needs_refresh);
    assert_eq!(state.reason, "invalid_jwt");
    assert!(state.session_id.is_empty());
    assert!(state.organization_id.is_empty());
    assert!(state.role.is_empty());
    assert!(state.permissions.is_empty());
    assert!(state.entitlements.is_empty());
    assert!(state.user.is_none());
    assert!(state.impersonator.is_none());
}

#[tokio::test]
async fn valid_signed_session_and_matching_user_authenticate_and_cache_across_client_clones() {
    let (server, client) = setup().await;
    serve_keys(&server, json!({"keys":[keys()["trusted"]["jwk"]]}), 1).await;
    let data = session(token(&claims(), "trusted", Some("trusted")));
    for client in [client.clone(), client.clone()] {
        let state = authenticate(&client, &data).await;
        assert!(state.authenticated);
        assert_eq!(state.user.unwrap().id, "user_1");
        assert_eq!(state.session_id, "sess_1");
        assert_eq!(state.organization_id, "org_1");
        assert_eq!(state.role, "admin");
        assert_eq!(state.permissions, ["read"]);
        assert_eq!(state.entitlements, ["enterprise"]);
        assert!(state.impersonator.is_none());
    }
}

#[tokio::test]
async fn captured_valid_jwt_with_mismatched_cookie_user_is_rejected() {
    let (server, client) = setup().await;
    serve_keys(&server, json!({"keys":[keys()["trusted"]["jwk"]]}), 1).await;
    let mut data = session(token(&claims(), "trusted", Some("trusted")));
    data.user = Some(user("user_victim"));
    assert_rejected(authenticate(&client, &data).await);
}

#[tokio::test]
async fn forged_cookie_impersonator_without_signed_claim_is_rejected() {
    let (server, client) = setup().await;
    serve_keys(&server, json!({"keys":[keys()["trusted"]["jwk"]]}), 1).await;
    let mut data = session(token(&claims(), "trusted", Some("trusted")));
    data.impersonator =
        Some(serde_json::from_value(json!({"email":"attacker@example.com"})).unwrap());
    assert_rejected(authenticate(&client, &data).await);
}

#[tokio::test]
async fn signed_impersonator_is_authoritative_with_matching_or_absent_cookie_claim() {
    let (server, client) = setup().await;
    serve_keys(&server, json!({"keys":[keys()["trusted"]["jwk"]]}), 1).await;
    let mut claims = claims();
    claims["impersonator"] = json!({"email":"admin@example.com", "reason":"support"});
    let mut data = session(token(&claims, "trusted", Some("trusted")));
    for cookie in [
        None,
        Some(serde_json::from_value(claims["impersonator"].clone()).unwrap()),
    ] {
        data.impersonator = cookie;
        let state = authenticate(&client, &data).await;
        assert!(state.authenticated);
        assert_eq!(state.user.unwrap().id, "user_1");
        let impersonator = state.impersonator.unwrap();
        assert_eq!(impersonator.email, "admin@example.com");
        assert_eq!(impersonator.reason.as_deref(), Some("support"));
    }
}

#[tokio::test]
async fn mismatched_cookie_impersonator_email_or_reason_is_rejected() {
    let (server, client) = setup().await;
    serve_keys(&server, json!({"keys":[keys()["trusted"]["jwk"]]}), 1).await;
    let mut claims = claims();
    claims["impersonator"] = json!({"email":"admin@example.com", "reason":"support"});
    let mut data = session(token(&claims, "trusted", Some("trusted")));
    for cookie in [
        json!({"email":"attacker@example.com", "reason":"support"}),
        json!({"email":"admin@example.com", "reason":"forged"}),
    ] {
        data.impersonator = Some(serde_json::from_value(cookie).unwrap());
        assert_rejected(authenticate(&client, &data).await);
    }
}

#[tokio::test]
async fn only_verified_expired_tokens_signal_refresh_and_logout_uses_verified_sid() {
    let (server, client) = setup().await;
    serve_keys(&server, json!({"keys":[keys()["trusted"]["jwk"]]}), 1).await;
    let mut claims = claims();
    claims["exp"] = json!(1);
    let data = session(token(&claims, "trusted", Some("trusted")));
    let state = authenticate(&client, &data).await;
    assert!(!state.authenticated);
    assert!(state.needs_refresh);
    assert_eq!(state.reason, "session_expired");
    let sealed = seal_session(&data, PASSWORD).unwrap();
    assert!(
        client
            .session(sealed, PASSWORD)
            .logout_url(None)
            .await
            .unwrap()
            .contains("session_id=sess_1")
    );
    let data = session(token(&claims, "attacker", Some("trusted")));
    assert_rejected(authenticate(&client, &data).await);
    let sealed = seal_session(&data, PASSWORD).unwrap();
    assert!(
        client
            .session(sealed, PASSWORD)
            .logout_url(None)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn forged_malformed_unsigned_and_wrong_algorithm_tokens_fail_closed() {
    let (server, client) = setup().await;
    serve_keys(&server, json!({"keys":[keys()["trusted"]["jwk"]]}), 1).await;
    let signed = token(&claims(), "trusted", Some("trusted"));
    let parts: Vec<_> = signed.split('.').collect();
    let mut forged = claims();
    forged["sub"] = json!("victim");
    forged["permissions"] = json!(["*"]);
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&forged).unwrap());
    let tampered = format!("{}.{payload}.{}", parts[0], parts[2]);
    let none = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","kid":"trusted"}"#);
    let hmac = encode(
        &Header::new(Algorithm::HS256),
        &claims(),
        &EncodingKey::from_secret(b"cookie-password"),
    )
    .unwrap();
    for jwt in [
        "".into(),
        "garbage".into(),
        format!("h.{payload}.s"),
        format!("{none}.{payload}."),
        hmac,
        tampered,
        token(&claims(), "attacker", Some("trusted")),
        token(&claims(), "trusted", None),
    ] {
        assert_rejected(authenticate(&client, &session(jwt)).await);
    }
}

#[tokio::test]
async fn missing_or_invalid_exp_subject_and_future_nbf_are_rejected() {
    let (server, client) = setup().await;
    serve_keys(&server, json!({"keys":[keys()["trusted"]["jwk"]]}), 1).await;
    for (field, value) in [
        ("exp", None),
        ("exp", Some(json!("not-a-time"))),
        ("sub", None),
        ("sub", Some(json!(""))),
        ("nbf", Some(json!(9999999999u64))),
    ] {
        let mut claims = claims();
        match value {
            Some(value) => {
                claims[field] = value;
            }
            None => {
                claims.as_object_mut().unwrap().remove(field);
            }
        }
        assert_rejected(
            authenticate(
                &client,
                &session(token(&claims, "trusted", Some("trusted"))),
            )
            .await,
        );
    }
}

#[tokio::test]
async fn unknown_kid_refreshes_once_and_rejects_if_still_missing() {
    let (server, client) = setup().await;
    serve_keys(&server, json!({"keys":[keys()["trusted"]["jwk"]]}), 2).await;
    assert_rejected(
        authenticate(
            &client,
            &session(token(&claims(), "attacker", Some("unknown"))),
        )
        .await,
    );
}

#[tokio::test]
async fn rotated_key_is_loaded_after_cache_miss() {
    let (server, client) = setup().await;
    serve_keys(&server, json!({"keys":[keys()["trusted"]["jwk"]]}), 1).await;
    assert!(
        authenticate(
            &client,
            &session(token(&claims(), "trusted", Some("trusted")))
        )
        .await
        .authenticated
    );
    server.reset().await;
    serve_keys(&server, json!({"keys":[keys()["attacker"]["jwk"]]}), 1).await;
    assert!(
        authenticate(
            &client,
            &session(token(&claims(), "attacker", Some("attacker")))
        )
        .await
        .authenticated
    );
}

#[tokio::test]
async fn failed_unknown_kid_refresh_preserves_cached_keys_and_allows_later_rotation() {
    for response in [
        ResponseTemplate::new(503),
        ResponseTemplate::new(200).set_body_string("not json"),
    ] {
        let (server, client) = setup().await;
        let valid = session(token(&claims(), "trusted", Some("trusted")));
        let rotated = session(token(&claims(), "attacker", Some("attacker")));
        serve_keys(&server, json!({"keys":[keys()["trusted"]["jwk"]]}), 1).await;
        assert!(authenticate(&client, &valid).await.authenticated);
        server.reset().await;
        Mock::given(method("GET"))
            .and(path("/sso/jwks/client_test"))
            .respond_with(response.set_delay(std::time::Duration::from_millis(50)))
            .expect(1)
            .mount(&server)
            .await;

        let clients = [client.clone(), client.clone(), client.clone()];
        let states = futures_util::future::join_all(
            clients.iter().map(|client| authenticate(client, &rotated)),
        )
        .await;
        for state in states {
            assert_rejected(state);
        }
        // Even while the endpoint is broken, the shared cached key still works.
        assert!(authenticate(&client.clone(), &valid).await.authenticated);
        server.reset().await;
        serve_keys(&server, json!({"keys":[keys()["attacker"]["jwk"]]}), 1).await;
        assert!(authenticate(&client, &rotated).await.authenticated);
    }
}

#[tokio::test]
async fn concurrent_rotated_sessions_share_one_refresh_across_client_clones() {
    let (server, client) = setup().await;
    serve_keys(&server, json!({"keys":[keys()["trusted"]["jwk"]]}), 1).await;
    let valid = session(token(&claims(), "trusted", Some("trusted")));
    assert!(authenticate(&client, &valid).await.authenticated);
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/sso/jwks/client_test"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"keys":[keys()["attacker"]["jwk"]]}))
                .set_delay(std::time::Duration::from_millis(50)),
        )
        .expect(1)
        .mount(&server)
        .await;
    let rotated = session(token(&claims(), "attacker", Some("attacker")));
    let clients = [client.clone(), client.clone(), client.clone()];
    let states =
        futures_util::future::join_all(clients.iter().map(|client| authenticate(client, &rotated)))
            .await;
    assert!(states.iter().all(|state| state.authenticated));
}

#[tokio::test]
async fn unavailable_malformed_or_incompatible_jwks_fail_closed() {
    for response in [
        ResponseTemplate::new(503),
        ResponseTemplate::new(200).set_body_string("not json"),
        ResponseTemplate::new(200)
            .set_body_json(json!({"keys":[{"kid":"trusted","kty":"RSA","n":"bad","e":"bad"}]})),
        ResponseTemplate::new(200)
            .set_body_json(json!({"keys":[{"kid":"trusted","kty":"oct","k":"c2VjcmV0"}]})),
    ] {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .respond_with(response)
            .mount(&server)
            .await;
        assert_rejected(
            authenticate(
                &client,
                &session(token(&claims(), "trusted", Some("trusted"))),
            )
            .await,
        );
    }
}

#[tokio::test]
async fn missing_client_id_fails_closed_without_fetching_jwks() {
    let server = MockServer::start().await;
    let client = Client::builder().base_url(server.uri()).build();
    assert_rejected(
        authenticate(
            &client,
            &session(token(&claims(), "trusted", Some("trusted"))),
        )
        .await,
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn ambiguous_or_non_signing_keys_are_rejected() {
    for (field, value) in [
        ("alg", json!("HS256")),
        ("use", json!("enc")),
        ("key_ops", json!(["encrypt"])),
        ("key_ops", json!("verify")),
        ("duplicate", json!(true)),
    ] {
        let (server, client) = setup().await;
        let mut key = keys()["trusted"]["jwk"].clone();
        let jwks = if field == "duplicate" {
            json!({"keys":[key.clone(), key]})
        } else {
            key[field] = value;
            json!({"keys":[key]})
        };
        serve_keys(&server, jwks, 1).await;
        assert_rejected(
            authenticate(
                &client,
                &session(token(&claims(), "trusted", Some("trusted"))),
            )
            .await,
        );
    }
}

#[tokio::test]
async fn jwks_cache_expires_at_configured_ttl() {
    let (server, client) = setup().await;
    serve_keys(&server, json!({"keys":[keys()["trusted"]["jwk"]]}), 2).await;
    let helper = client.jwks().with_ttl(std::time::Duration::ZERO);
    helper.fetch().await.unwrap();
    helper.fetch().await.unwrap();
}
