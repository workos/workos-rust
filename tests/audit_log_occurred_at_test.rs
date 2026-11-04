// @oagen-ignore-file
use serde_json::{Value, json};
use workos::models::AuditLogEvent;

fn event_without_timestamp() -> Value {
    json!({
        "action": "user.signed_in",
        "actor": {"id": "user_1", "type": "user"},
        "targets": [],
        "context": {"location": "127.0.0.1"}
    })
}

#[test]
fn omitted_timestamp_deserializes_and_stays_absent() -> Result<(), serde_json::Error> {
    let event: AuditLogEvent = serde_json::from_value(event_without_timestamp())?;

    assert!(event.occurred_at.is_empty());
    let serialized = serde_json::to_value(event)?;
    assert!(serialized.get("occurred_at").is_none());
    Ok(())
}

#[test]
fn existing_string_assignment_preserves_explicit_timestamp() -> Result<(), serde_json::Error> {
    let mut event: AuditLogEvent = serde_json::from_value(event_without_timestamp())?;
    event.occurred_at = "2026-11-03T12:34:56.789Z".to_string();

    let serialized = serde_json::to_value(event)?;
    assert_eq!(serialized["occurred_at"], "2026-11-03T12:34:56.789Z");
    Ok(())
}
