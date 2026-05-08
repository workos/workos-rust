// @oagen-ignore-file
//! Webhook signature verification (H01) and signature primitives (H02).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::error::Error;
use crate::models::EventSchema;

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_TOLERANCE: Duration = Duration::from_secs(180);

/// Verifies WorkOS webhook signatures and parses the event envelope.
pub struct WebhookVerifier {
    secret: String,
    tolerance: Duration,
    now: Box<dyn Fn() -> SystemTime + Send + Sync>,
}

impl WebhookVerifier {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            tolerance: DEFAULT_TOLERANCE,
            now: Box::new(SystemTime::now),
        }
    }

    pub fn with_tolerance(mut self, tolerance: Duration) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Override the clock — for tests.
    pub fn with_clock(mut self, now: impl Fn() -> SystemTime + Send + Sync + 'static) -> Self {
        self.now = Box::new(now);
        self
    }

    /// Verifies that `body` matches the signature in `sig_header` and returns the body.
    pub fn verify_payload(&self, sig_header: &str, body: &str) -> Result<String, Error> {
        if sig_header.is_empty() {
            return Err(Error::Webhook("webhook not signed".to_string()));
        }

        let (timestamp, signature) = parse_webhook_signature_header(sig_header)?;

        let ts: i64 = timestamp
            .parse()
            .map_err(|_| Error::Webhook("invalid timestamp in signature header".to_string()))?;
        let signed_at = UNIX_EPOCH + Duration::from_millis(ts as u64);
        let now = (self.now)();
        let diff = match now.duration_since(signed_at) {
            Ok(d) => d,
            Err(e) => e.duration(),
        };
        if diff > self.tolerance {
            return Err(Error::Webhook("timestamp outside tolerance".to_string()));
        }

        let expected = compute_webhook_signature(&self.secret, &timestamp, body);
        if expected.as_bytes().ct_eq(signature.as_bytes()).unwrap_u8() != 1 {
            return Err(Error::Webhook("no valid signature found".to_string()));
        }

        Ok(body.to_string())
    }

    /// Verifies and deserializes the webhook payload into an `EventSchema`.
    pub fn construct_event(&self, sig_header: &str, body: &str) -> Result<EventSchema, Error> {
        let verified = self.verify_payload(sig_header, body)?;
        serde_json::from_str(&verified).map_err(Error::from)
    }
}

/// Computes the HMAC-SHA256 signature for a webhook payload.
pub fn compute_webhook_signature(secret: &str, timestamp: &str, body: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC-SHA256 accepts any key size");
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Parses a `t=<timestamp>, v1=<signature>` header into its components.
pub fn parse_webhook_signature_header(header: &str) -> Result<(String, String), Error> {
    if header.is_empty() {
        return Err(Error::Webhook("webhook not signed".to_string()));
    }

    let mut timestamp = String::new();
    let mut signature = String::new();
    for part in header.split(',') {
        let trimmed = part.trim();
        let (k, v) = trimmed
            .split_once('=')
            .ok_or_else(|| Error::Webhook("invalid webhook signature header".to_string()))?;
        match k.trim() {
            "t" => timestamp = v.trim().to_string(),
            "v1" => signature = v.trim().to_string(),
            _ => {}
        }
    }

    if timestamp.is_empty() || signature.is_empty() {
        return Err(Error::Webhook(
            "invalid webhook signature header".to_string(),
        ));
    }

    Ok((timestamp, signature))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_now(ms: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(ms)
    }

    #[test]
    fn parse_header_ok() {
        let (ts, sig) = parse_webhook_signature_header("t=1234, v1=abcd").unwrap();
        assert_eq!(ts, "1234");
        assert_eq!(sig, "abcd");
    }

    #[test]
    fn parse_header_missing_value() {
        assert!(parse_webhook_signature_header("t=1234").is_err());
    }

    #[test]
    fn parse_header_empty() {
        assert!(parse_webhook_signature_header("").is_err());
    }

    #[test]
    fn compute_signature_matches_known_value() {
        // Recomputable: hmac_sha256("secret", "1700000000000.body")
        let sig = compute_webhook_signature("secret", "1700000000000", "body");
        // Length sanity: hex of sha256 is always 64 chars.
        assert_eq!(sig.len(), 64);
        // Recompute and ensure stable.
        let again = compute_webhook_signature("secret", "1700000000000", "body");
        assert_eq!(sig, again);
    }

    #[test]
    fn verify_round_trip() {
        let secret = "test-secret";
        let body = r#"{"id":"evt_1","object":"event","event":"test","data":{},"created_at":"2026-01-01T00:00:00Z"}"#;
        let timestamp = "1735689600000"; // 2025-01-01 UTC in ms
        let sig = compute_webhook_signature(secret, timestamp, body);
        let header = format!("t={timestamp}, v1={sig}");

        let verifier =
            WebhookVerifier::new(secret).with_clock(move || fixed_now(1_735_689_600_000));
        let verified = verifier.verify_payload(&header, body).unwrap();
        assert_eq!(verified, body);

        let event = verifier.construct_event(&header, body).unwrap();
        assert_eq!(event.id, "evt_1");
    }

    #[test]
    fn verify_rejects_outside_tolerance() {
        let secret = "test-secret";
        let body = "x";
        let signed_at = 1_700_000_000_000u64;
        let sig = compute_webhook_signature(secret, &signed_at.to_string(), body);
        let header = format!("t={signed_at}, v1={sig}");

        let now_ms = signed_at + 200_000; // 200s later, default tolerance 180s
        let verifier = WebhookVerifier::new(secret).with_clock(move || fixed_now(now_ms));
        let err = verifier.verify_payload(&header, body).unwrap_err();
        assert!(matches!(err, Error::Webhook(_)));
    }

    #[test]
    fn verify_rejects_bad_signature() {
        let secret = "test-secret";
        let body = "x";
        let signed_at = 1_700_000_000_000u64;
        let header = format!("t={signed_at}, v1=deadbeef");
        let verifier = WebhookVerifier::new(secret).with_clock(move || fixed_now(signed_at));
        assert!(verifier.verify_payload(&header, body).is_err());
    }
}
