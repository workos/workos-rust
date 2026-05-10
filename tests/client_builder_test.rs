// @oagen-ignore-file
//! Locks in `ClientBuilder::try_build` validation.

use workos::{Client, Error};

#[test]
fn try_build_succeeds_with_valid_inputs() {
    let result = Client::builder()
        .api_key("sk_test_abc")
        .user_agent("my-app/1.0")
        .try_build();
    assert!(result.is_ok());
}

#[test]
fn try_build_rejects_api_key_with_control_byte() {
    // HTTP header values cannot contain bare CR / NUL bytes. The legacy
    // `build()` silently dropped such keys and produced an unauthenticated
    // client; `try_build` surfaces them as `Error::Builder`.
    let result = Client::builder().api_key("bad\nkey").try_build();
    assert!(matches!(result, Err(Error::Builder(_))));
}

#[test]
fn try_build_rejects_user_agent_with_control_byte() {
    let result = Client::builder().user_agent("bad\rua").try_build();
    assert!(matches!(result, Err(Error::Builder(_))));
}

#[test]
fn try_build_allows_empty_api_key_for_public_flows() {
    // OAuth helpers and PKCE flows can run without an API key. `try_build`
    // must permit that — only invalid bytes should fail.
    let result = Client::builder().try_build();
    assert!(result.is_ok());
}
