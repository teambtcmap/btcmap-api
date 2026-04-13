// Lands in isolation ahead of the endpoints that will consume it. The next
// PR wires `NostrAuth` into `POST /v4/auth/nostr`; until then clippy sees
// these items as dead.
#![allow(dead_code)]

use crate::error::Error;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use nostr::event::Event;
use nostr::nips::nip19::ToBech32;
use nostr::JsonUtil;
use nostr::Kind;
use std::time::{SystemTime, UNIX_EPOCH};

const NIP98_KIND: u16 = 27235;
const MAX_TIMESTAMP_DRIFT_SECS: u64 = 60;

/// Result of a successfully verified NIP-98 event.
#[derive(Debug)]
pub struct VerifiedNip98Event {
    /// Bech32-encoded (`npub1...`) Nostr public key, per NIP-19. Matches the
    /// encoding used by the `user.npub` column (see `select_by_npub` tests).
    pub npub: String,
}

/// Verify a NIP-98 HTTP auth event.
///
/// `authorization_payload` is the base64-encoded event string (the part after
/// "Nostr " in the Authorization header).
///
/// Validates:
/// 1. Base64 decoding and JSON parsing
/// 2. `kind == 27235`
/// 3. `created_at` within 60 seconds of server time
/// 4. `u` tag matches `expected_url`
/// 5. `method` tag matches `expected_method` (case-insensitive)
/// 6. Schnorr signature is valid
pub fn verify(
    authorization_payload: &str,
    expected_url: &str,
    expected_method: &str,
) -> Result<VerifiedNip98Event, Error> {
    let decoded = BASE64
        .decode(authorization_payload)
        .map_err(|e| Error::Other(format!("Invalid base64: {e}")))?;

    let json_str =
        String::from_utf8(decoded).map_err(|e| Error::Other(format!("Invalid UTF-8: {e}")))?;

    let event = Event::from_json(&json_str)
        .map_err(|e| Error::Other(format!("Invalid Nostr event JSON: {e}")))?;

    if event.kind != Kind::from_u16(NIP98_KIND) {
        return Err(Error::Other(format!(
            "Invalid event kind: expected {NIP98_KIND}, got {}",
            event.kind.as_u16()
        )));
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Error::Other(format!("System time error: {e}")))?
        .as_secs();
    let event_time = event.created_at.as_secs();
    let drift = now.abs_diff(event_time);
    if drift > MAX_TIMESTAMP_DRIFT_SECS {
        return Err(Error::Other(format!(
            "Event timestamp too far from server time (drift: {drift}s, max: {MAX_TIMESTAMP_DRIFT_SECS}s)"
        )));
    }

    let u_tag = find_tag_value(&event, "u")
        .ok_or_else(|| Error::Other("Missing 'u' tag in NIP-98 event".to_string()))?;
    if u_tag != expected_url {
        return Err(Error::Other(format!(
            "URL mismatch: event has '{u_tag}', expected '{expected_url}'"
        )));
    }

    let method_tag = find_tag_value(&event, "method")
        .ok_or_else(|| Error::Other("Missing 'method' tag in NIP-98 event".to_string()))?;
    if !method_tag.eq_ignore_ascii_case(expected_method) {
        return Err(Error::Other(format!(
            "Method mismatch: event has '{method_tag}', expected '{expected_method}'"
        )));
    }

    event
        .verify()
        .map_err(|e| Error::Other(format!("Signature verification failed: {e}")))?;

    let npub = event
        .pubkey
        .to_bech32()
        .map_err(|e| Error::Other(format!("Pubkey bech32 encoding failed: {e}")))?;

    Ok(VerifiedNip98Event { npub })
}

/// Extract the base64 payload from an `Authorization` header value. Matches
/// the `Nostr` scheme case-insensitively per RFC 9110.
pub fn extract_nostr_auth(authorization_header: &str) -> Option<&str> {
    let (scheme, rest) = authorization_header.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("Nostr") {
        Some(rest)
    } else {
        None
    }
}

fn find_tag_value(event: &Event, tag_name: &str) -> Option<String> {
    for tag in event.tags.iter() {
        let tag_vec = tag.as_slice();
        if tag_vec.len() >= 2 && tag_vec[0] == tag_name {
            return Some(tag_vec[1].to_string());
        }
    }
    None
}

#[cfg(test)]
mod test {
    use super::*;
    use nostr::event::EventBuilder;
    use nostr::key::Keys;
    use nostr::Tag;
    use nostr::Timestamp;

    fn make_nip98_event(keys: &Keys, url: &str, method: &str) -> String {
        let event = EventBuilder::new(Kind::from_u16(NIP98_KIND), "")
            .tags(vec![
                Tag::parse(["u", url]).unwrap(),
                Tag::parse(["method", method]).unwrap(),
            ])
            .custom_created_at(Timestamp::now())
            .sign_with_keys(keys)
            .unwrap();
        let json = event.as_json();
        BASE64.encode(json.as_bytes())
    }

    #[test]
    fn valid_event_returns_bech32_npub() {
        let keys = Keys::generate();
        let url = "https://api.btcmap.org/v4/auth/nostr";
        let b64 = make_nip98_event(&keys, url, "POST");
        let result = verify(&b64, url, "POST").unwrap();
        assert!(
            result.npub.starts_with("npub1"),
            "npub should be bech32-encoded, got {}",
            result.npub
        );
        assert_eq!(result.npub, keys.public_key().to_bech32().unwrap());
    }

    #[test]
    fn method_match_is_case_insensitive() {
        let keys = Keys::generate();
        let url = "https://api.btcmap.org/v4/auth/nostr";
        let b64 = make_nip98_event(&keys, url, "post");
        assert!(verify(&b64, url, "POST").is_ok());
    }

    #[test]
    fn wrong_url() {
        let keys = Keys::generate();
        let b64 = make_nip98_event(&keys, "https://example.com/wrong", "POST");
        let err = verify(&b64, "https://api.btcmap.org/v4/auth/nostr", "POST").unwrap_err();
        assert!(err.to_string().contains("URL mismatch"));
    }

    #[test]
    fn wrong_method() {
        let keys = Keys::generate();
        let url = "https://api.btcmap.org/v4/auth/nostr";
        let b64 = make_nip98_event(&keys, url, "GET");
        let err = verify(&b64, url, "POST").unwrap_err();
        assert!(err.to_string().contains("Method mismatch"));
    }

    #[test]
    fn expired_event() {
        let keys = Keys::generate();
        let url = "https://api.btcmap.org/v4/auth/nostr";
        let event = EventBuilder::new(Kind::from_u16(NIP98_KIND), "")
            .tags(vec![
                Tag::parse(["u", url]).unwrap(),
                Tag::parse(["method", "POST"]).unwrap(),
            ])
            .custom_created_at(Timestamp::from(0))
            .sign_with_keys(&keys)
            .unwrap();
        let b64 = BASE64.encode(event.as_json().as_bytes());
        let err = verify(&b64, url, "POST").unwrap_err();
        assert!(err.to_string().contains("timestamp too far"));
    }

    #[test]
    fn wrong_kind() {
        let keys = Keys::generate();
        let url = "https://api.btcmap.org/v4/auth/nostr";
        let event = EventBuilder::new(Kind::from_u16(1), "")
            .tags(vec![
                Tag::parse(["u", url]).unwrap(),
                Tag::parse(["method", "POST"]).unwrap(),
            ])
            .custom_created_at(Timestamp::now())
            .sign_with_keys(&keys)
            .unwrap();
        let b64 = BASE64.encode(event.as_json().as_bytes());
        let err = verify(&b64, url, "POST").unwrap_err();
        assert!(err.to_string().contains("Invalid event kind"));
    }

    #[test]
    fn invalid_base64() {
        let err = verify("not-valid-base64!!!", "https://example.com", "POST").unwrap_err();
        assert!(err.to_string().contains("Invalid base64"));
    }

    #[test]
    fn tampered_signature_rejected() {
        // Build a valid event, then flip one hex nibble of its signature so
        // the structure and tags remain valid but the Schnorr check fails.
        let keys = Keys::generate();
        let url = "https://api.btcmap.org/v4/auth/nostr";
        let event = EventBuilder::new(Kind::from_u16(NIP98_KIND), "")
            .tags(vec![
                Tag::parse(["u", url]).unwrap(),
                Tag::parse(["method", "POST"]).unwrap(),
            ])
            .custom_created_at(Timestamp::now())
            .sign_with_keys(&keys)
            .unwrap();

        // Flip one character of the sig hex. The event's `id` still matches
        // the content, the tags are valid, only the signature is wrong.
        let mut json: serde_json::Value = serde_json::from_str(&event.as_json()).unwrap();
        let sig = json["sig"].as_str().unwrap().to_string();
        let first = sig.chars().next().unwrap();
        let flipped = if first == '0' { '1' } else { '0' };
        let mut new_sig = flipped.to_string();
        new_sig.push_str(&sig[1..]);
        json["sig"] = serde_json::Value::String(new_sig);
        let tampered_json = serde_json::to_string(&json).unwrap();
        let b64 = BASE64.encode(tampered_json.as_bytes());

        let err = verify(&b64, url, "POST").unwrap_err();
        let msg = err.to_string();
        // `event.verify()` checks both the id and the signature; either
        // failing here is acceptable — we just need the verifier to reject.
        assert!(
            msg.contains("Signature verification failed")
                || msg.contains("Invalid Nostr event JSON"),
            "expected rejection, got: {msg}"
        );
    }

    #[test]
    fn extract_nostr_auth_uppercase_scheme() {
        let header = "Nostr eyJpZCI6IjEyMyJ9";
        assert_eq!(extract_nostr_auth(header), Some("eyJpZCI6IjEyMyJ9"));
    }

    #[test]
    fn extract_nostr_auth_lowercase_scheme() {
        let header = "nostr eyJpZCI6IjEyMyJ9";
        assert_eq!(extract_nostr_auth(header), Some("eyJpZCI6IjEyMyJ9"));
    }

    #[test]
    fn extract_nostr_auth_rejects_bearer() {
        assert_eq!(extract_nostr_auth("Bearer some-token"), None);
    }

    #[test]
    fn extract_nostr_auth_rejects_no_space() {
        assert_eq!(extract_nostr_auth("Nostr"), None);
    }
}
