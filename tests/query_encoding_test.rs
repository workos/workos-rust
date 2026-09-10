// @oagen-ignore-file
//! Query encoding must preserve map namespaces without changing Serde flattening.

use std::collections::HashMap;

use serde::Serialize;
use serde_json::json;
use workos::{Client, query::encode_query};

#[test]
fn nested_maps_keep_their_full_namespace() -> Result<(), workos::Error> {
    let query = encode_query(&json!({
        "redirect_uri": "https://app.example/callback",
        "provider_query_params": {
            "redirect_uri": "https://attacker.example/callback",
            "nested": { "state": "attacker-state" },
            "scope": ["openid", "email"]
        }
    }))?;
    assert_eq!(
        query,
        concat!(
            "provider_query_params%5Bnested%5D%5Bstate%5D=attacker-state&",
            "provider_query_params%5Bredirect_uri%5D=https%3A%2F%2Fattacker.example%2Fcallback&",
            "provider_query_params%5Bscope%5D=openid&provider_query_params%5Bscope%5D=email&",
            "redirect_uri=https%3A%2F%2Fapp.example%2Fcallback"
        )
    );
    Ok(())
}

#[test]
fn empty_map_keys_do_not_reset_the_namespace() -> Result<(), workos::Error> {
    assert_eq!(
        encode_query(&json!({ "": { "state": "nested" } }))?,
        "%5Bstate%5D=nested"
    );
    Ok(())
}

#[test]
fn flattened_enum_fields_stay_top_level() -> Result<(), workos::Error> {
    #[derive(Serialize)]
    struct Params {
        #[serde(flatten)]
        target: workos::authorization::ResourceTarget,
        limit: u32,
    }
    let params = Params {
        target: workos::authorization::ResourceTarget::ByExternalId {
            resource_external_id: "external_123".into(),
            resource_type_slug: "document".into(),
        },
        limit: 10,
    };
    assert_eq!(
        encode_query(&params)?,
        "limit=10&resource_external_id=external_123&resource_type_slug=document"
    );
    Ok(())
}

#[test]
fn scalar_array_and_empty_values_keep_existing_encoding() -> Result<(), workos::Error> {
    assert_eq!(
        encode_query(&json!({
            "array": ["one", "two"],
            "bool": true,
            "empty_array": [],
            "empty_map": {},
            "null": null,
            "number": 42,
            "text": "a +&=é"
        }))?,
        "array=one&array=two&bool=true&number=42&text=a+%2B%26%3D%C3%A9"
    );
    assert_eq!(encode_query(&json!({}))?, "");
    assert_eq!(encode_query(&())?, "");
    Ok(())
}

fn provider_params() -> HashMap<String, String> {
    HashMap::from([
        (
            "redirect_uri".into(),
            "https://attacker.example/callback".into(),
        ),
        ("client_id".into(), "attacker-client".into()),
        ("response_type".into(), "token".into()),
        ("state".into(), "attacker-state".into()),
        ("prompt".into(), "select_account".into()),
        (
            "custom&key=+#[]é".into(),
            "value&state=injected +#[]é".into(),
        ),
    ])
}

fn assert_authorization_query(url: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = url::Url::parse(url)?;
    assert_eq!(url.path(), path);
    // Keep pairs rather than collecting into a map, so duplicates cannot be hidden.
    let pairs: Vec<_> = url.query_pairs().collect();
    for (key, value) in [
        ("redirect_uri", "https://app.example/callback"),
        ("client_id", "client_trusted"),
        ("response_type", "code"),
        ("state", "trusted-state"),
        ("provider_scopes", "openid,email"),
    ] {
        let values: Vec<_> = pairs
            .iter()
            .filter(|(name, _)| name == key)
            .map(|(_, value)| value.as_ref())
            .collect();
        assert_eq!(values, [value], "unexpected values for {key}");
    }
    for (key, value) in provider_params() {
        let key = format!("provider_query_params[{key}]");
        let values: Vec<_> = pairs
            .iter()
            .filter(|(name, _)| name == &key)
            .map(|(_, value)| value.as_ref())
            .collect();
        assert_eq!(values, [value.as_str()], "missing provider parameter {key}");
    }
    assert_eq!(pairs.len(), 5 + provider_params().len());
    Ok(())
}

#[test]
fn sso_provider_params_cannot_inject_authorization_params() -> Result<(), Box<dyn std::error::Error>>
{
    let client = Client::builder().client_id("client_trusted").build();
    let mut params = workos::sso::GetAuthorizationUrlParams::new("https://app.example/callback");
    params.state = Some("trusted-state".into());
    params.provider_scopes = Some(vec!["openid".into(), "email".into()]);
    params.provider_query_params = Some(provider_params());
    let url = client.sso().get_authorization_url(params)?;
    assert_authorization_query(&url, "/sso/authorize")
}

#[test]
fn user_management_provider_params_cannot_inject_authorization_params()
-> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder().client_id("client_trusted").build();
    let mut params =
        workos::user_management::GetAuthorizationUrlParams::new("https://app.example/callback");
    params.state = Some("trusted-state".into());
    params.provider_scopes = Some(vec!["openid".into(), "email".into()]);
    params.provider_query_params = Some(provider_params());
    let url = client.user_management().get_authorization_url(params)?;
    assert_authorization_query(&url, "/user_management/authorize")
}
