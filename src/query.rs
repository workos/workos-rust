// @oagen-ignore-file
//! Query-string encoding helpers shared by generated resource code.
//!
//! The default `serde_urlencoded` crate rejects `Vec` fields outright, which
//! makes it unusable for our generated query-param structs. This module
//! provides:
//!
//! - [`encode_query`] — encodes any `Serialize` value as a URL query string,
//!   supporting arrays (repeated keys by default), nested objects produced by
//!   `#[serde(flatten)]`, and field-level `serialize_with` overrides.
//! - [`serialize_comma_separated`] / [`serialize_comma_separated_opt`] — used
//!   by generated code for fields whose OpenAPI spec marks
//!   `style: form, explode: false` (i.e. the comma-joined wire form).

use serde::{Serialize, Serializer};

use crate::error::Error;

/// Serialize a `Vec<T>` as a single comma-joined query string value.
///
/// Used by generated query-param structs whose OpenAPI spec marks the field
/// with `style: form, explode: false` (the comma-joined form). The values are
/// rendered via `serde_json::to_string` and then unquoted for strings; this
/// matches the wire format the API expects without depending on `Display`.
///
/// When the option is `None`, the field is skipped (combine with
/// `#[serde(skip_serializing_if = "Option::is_none")]`).
#[doc(hidden)]
pub fn serialize_comma_separated_opt<S, T>(v: &Option<Vec<T>>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    match v {
        Some(values) => serialize_comma_separated(values, s),
        None => s.serialize_none(),
    }
}

/// Non-Option variant of [`serialize_comma_separated_opt`].
#[doc(hidden)]
pub fn serialize_comma_separated<S, T>(values: &[T], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    let mut parts: Vec<String> = Vec::with_capacity(values.len());
    for value in values {
        // Convert the value to JSON, then strip any surrounding quotes so the
        // wire-format reads `events=foo,bar` rather than `events="foo","bar"`.
        let json = serde_json::to_string(value).map_err(serde::ser::Error::custom)?;
        let trimmed = json
            .strip_prefix('"')
            .and_then(|t| t.strip_suffix('"'))
            .unwrap_or(&json);
        parts.push(trimmed.to_string());
    }
    s.serialize_str(&parts.join(","))
}

/// Encode a serializable value as a URL query string.
///
/// Replaces `serde_urlencoded::to_string` for our generated query-param
/// structs: it understands arrays (emitting one `key=value` pair per element
/// by default), nested objects (from `#[serde(flatten)]`, e.g. for
/// mutually-exclusive parameter group enums), and respects field-level
/// `serialize_with` overrides that already collapse arrays into strings.
///
/// Returns an empty string when the encoded form would have zero pairs.
pub fn encode_query<P: Serialize>(params: &P) -> Result<String, Error> {
    let value = serde_json::to_value(params)
        .map_err(|e| Error::Builder(format!("query encode failed: {e}")))?;
    let mut out = String::new();
    encode_value("", &value, &mut out);
    Ok(out)
}

/// Append `key=value` entries for `value` onto `out`.
///
/// - Object: recurses, using the property name as the key (top-level fields
///   produce `name=…`; nested objects from `serde(flatten)` merge into the
///   parent's namespace because the flatten-produced JSON object lives at the
///   parent level).
/// - Array: emits one `key=element` pair per element (repeated keys); empty
///   arrays produce no output.
/// - Null: skipped (matches `skip_serializing_if = "Option::is_none"` semantics).
/// - Scalars: percent-encoded under the given key.
fn encode_value(key: &str, value: &serde_json::Value, out: &mut String) {
    match value {
        serde_json::Value::Null => {}
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                encode_value(k, v, out);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                encode_value(key, item, out);
            }
        }
        serde_json::Value::String(s) => append_pair(out, key, s),
        serde_json::Value::Number(n) => append_pair(out, key, &n.to_string()),
        serde_json::Value::Bool(b) => append_pair(out, key, if *b { "true" } else { "false" }),
    }
}

fn append_pair(out: &mut String, key: &str, value: &str) {
    if !out.is_empty() {
        out.push('&');
    }
    out.push_str(&percent_encode_query(key));
    out.push('=');
    out.push_str(&percent_encode_query(value));
}

/// Application/x-www-form-urlencoded percent-encoding (encodes everything
/// except unreserved characters and replaces spaces with `+`).
fn percent_encode_query(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
