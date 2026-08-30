//! Parsing helpers for Codex authentication files.

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Extracts the email claim from an ID token without verifying it.
///
/// The token is read only from an already authenticated local Codex profile; it is never used to
/// authorize a request.
pub(crate) fn decode_jwt_email(id_token: &str) -> Option<String> {
    let payload = id_token.split('.').nth(1)?;
    let decoded = decode_jwt_payload(payload)?;
    let json: Value = serde_json::from_slice(&decoded).ok()?;

    json.get("email")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

/// Reads an auth file and returns its ID token's email claim when available.
pub(crate) fn email_from_auth_file(auth_path: &Path) -> Option<String> {
    let content = fs::read_to_string(auth_path).ok()?;
    let json: Value = serde_json::from_str(&content).ok()?;
    let id_token = json.get("tokens")?.get("id_token")?.as_str()?;
    decode_jwt_email(id_token)
}

fn decode_jwt_payload(payload: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(payload)
        .ok()
        .or_else(|| STANDARD.decode(payload).ok())
}

#[cfg(test)]
mod tests {
    use super::decode_jwt_email;

    #[test]
    fn reads_email_from_url_safe_jwt_payload() {
        let token = "header.eyJlbWFpbCI6InVzZXJAZXhhbXBsZS5jb20ifQ.signature";

        assert_eq!(decode_jwt_email(token).as_deref(), Some("user@example.com"));
    }

    #[test]
    fn rejects_a_token_without_a_payload() {
        assert_eq!(decode_jwt_email("header"), None);
    }
}
