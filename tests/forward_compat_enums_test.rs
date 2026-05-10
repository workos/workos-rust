// @oagen-ignore-file
//! Locks in the forward-compatible enum contract: unknown wire values must
//! deserialize into the SDK's fallback variant (preserving the original
//! string), and serializing/deserializing a known variant must round-trip.

use std::str::FromStr;
use workos::enums::ConnectionType;

#[test]
fn known_value_round_trips() {
    let value: ConnectionType = serde_json::from_str("\"GoogleOAuth\"").unwrap();
    assert_eq!(value, ConnectionType::GoogleOAuth);
    assert_eq!(value.as_str(), "GoogleOAuth");
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, "\"GoogleOAuth\"");
}

#[test]
fn unknown_value_preserved_through_unknown_variant() {
    let value: ConnectionType = serde_json::from_str("\"BrandNewConnectionType\"").unwrap();
    match &value {
        ConnectionType::Unknown(s) => assert_eq!(s, "BrandNewConnectionType"),
        other => panic!("expected Unknown, got {other:?}"),
    }
    // Re-serializing a fallback variant must echo the original wire string.
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, "\"BrandNewConnectionType\"");
}

#[test]
fn from_str_is_infallible_and_falls_back() {
    let v = ConnectionType::from_str("ADFSSAML").unwrap();
    assert_eq!(v, ConnectionType::Adfssaml);
    let v = ConnectionType::from_str("definitely-not-a-real-value").unwrap();
    assert_eq!(
        v,
        ConnectionType::Unknown("definitely-not-a-real-value".to_string())
    );
}

#[test]
fn display_emits_canonical_wire_value() {
    assert_eq!(ConnectionType::GoogleOAuth.to_string(), "GoogleOAuth");
    assert_eq!(
        ConnectionType::Unknown("Custom".to_string()).to_string(),
        "Custom"
    );
}
