<!-- @oagen-ignore-file -->
* **Security (VULN-1265), breaking:** Secret-bearing model fields and authentication parameters now use `SecretString` (or `Option<SecretString>`) instead of `String`, preventing credentials from appearing in derived `Debug` output. This includes vended credentials and full API keys, Vault plaintext, authentication/verification/device codes, PKCE verifiers, private keys, TOTP enrollment URI/QR data, and token-bearing invitation, reset, logout, and Admin Portal URLs.

  Migrate raw-string reads from `model.value` to `model.value.expose()` (or `.expose().to_owned()` when an owned `String` is required). For optional values, use `.as_ref().map(|value| value.expose())`; construct values with `.into()` or `SecretString::new(...)`. Serialization/deserialization remains transparent: JSON still contains ordinary strings, not redacted values. Only expose secrets where needed; do not log exposed values.

  This is an intentional breaking public-API change. Release coordination must select a major release via release-please; no package version is changed by this remediation.
