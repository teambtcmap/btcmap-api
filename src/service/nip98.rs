use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use nostr::event::Event;
use nostr::JsonUtil;
use nostr::Kind;
use std::time::{SystemTime, UNIX_EPOCH};

const NIP98_KIND: u16 = 27235;
const MAX_TIMESTAMP_DRIFT_SECS: u64 = 60;

/// Result of a successfully verified NIP-98 event.
#[derive(Debug)]
pub struct VerifiedNip98Event {
    /// Hex-encoded 32-byte Nostr public key.
    pub pubkey: String,
}

/// Verify a NIP-98 HTTP auth event.
///
/// The `authorization_payload` is the base64-encoded event string
/// (the part after "Nostr " in the Authorization header).
///
/// Validates:
/// 1. Base64 decoding and JSON parsing
/// 2. Kind == 27235
/// 3. created_at within 60 seconds of server time
/// 4. `u` tag matches expected_url
/// 5. `method` tag matches expected_method
/// 6. Schnorr signature is valid
pub fn verify(
    authorization_payload: &str,
    expected_url: &str,
    expected_method: &str,
) -> Result<VerifiedNip98Event, String> {
    // 1. Base64 decode
    let decoded = BASE64
        .decode(authorization_payload)
        .map_err(|e| format!("Invalid base64: {e}"))?;

    let json_str =
        String::from_utf8(decoded).map_err(|e| format!("Invalid UTF-8 in event: {e}"))?;

    // 2. Parse the Nostr event
    let event =
        Event::from_json(&json_str).map_err(|e| format!("Invalid Nostr event JSON: {e}"))?;

    // 3. Verify kind == 27235
    if event.kind != Kind::from_u16(NIP98_KIND) {
        return Err(format!(
            "Invalid event kind: expected {NIP98_KIND}, got {}",
            event.kind.as_u16()
        ));
    }

    // 4. Verify created_at within window
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System time error: {e}"))?
        .as_secs();
    let event_time = event.created_at.as_secs();
    let drift = now.abs_diff(event_time);
    if drift > MAX_TIMESTAMP_DRIFT_SECS {
        return Err(format!(
            "Event timestamp too far from server time (drift: {drift}s, max: {MAX_TIMESTAMP_DRIFT_SECS}s)"
        ));
    }

    // 5. Verify `u` tag matches expected URL
    let u_tag =
        find_tag_value(&event, "u").ok_or_else(|| "Missing 'u' tag in NIP-98 event".to_string())?;
    if u_tag != expected_url {
        return Err(format!(
            "URL mismatch: event has '{u_tag}', expected '{expected_url}'"
        ));
    }

    // 6. Verify `method` tag matches expected method
    let method_tag = find_tag_value(&event, "method")
        .ok_or_else(|| "Missing 'method' tag in NIP-98 event".to_string())?;
    if !method_tag.eq_ignore_ascii_case(expected_method) {
        return Err(format!(
            "Method mismatch: event has '{method_tag}', expected '{expected_method}'"
        ));
    }

    // 7. Verify Schnorr signature and event ID
    event
        .verify()
        .map_err(|e| format!("Signature verification failed: {e}"))?;

    Ok(VerifiedNip98Event {
        pubkey: event.pubkey.to_hex(),
    })
}

/// Extract the first value for a given single-letter tag.
fn find_tag_value(event: &Event, tag_name: &str) -> Option<String> {
    for tag in event.tags.iter() {
        let tag_vec = tag.as_slice();
        if tag_vec.len() >= 2 && tag_vec[0] == tag_name {
            return Some(tag_vec[1].to_string());
        }
    }
    None
}

/// Extract the base64 payload from an Authorization header value.
/// Returns None if the header doesn't use the "Nostr" scheme.
pub fn extract_nostr_auth(authorization_header: &str) -> Option<&str> {
    authorization_header.strip_prefix("Nostr ")
}

#[cfg(test)]
mod test {
    use super::*;
    use nostr::event::EventBuilder;
    use nostr::key::Keys;
    use nostr::Tag;
    use nostr::Timestamp;

    async fn make_nip98_event(keys: &Keys, url: &str, method: &str) -> String {
        let event = EventBuilder::new(Kind::from_u16(NIP98_KIND), "")
            .tags(vec![
                Tag::parse(["u", url]).unwrap(),
                Tag::parse(["method", method]).unwrap(),
            ])
            .custom_created_at(Timestamp::now())
            .sign(keys)
            .await
            .unwrap();
        let json = event.as_json();
        BASE64.encode(json.as_bytes())
    }

    #[actix_web::test]
    async fn valid_event() {
        let keys = Keys::generate();
        let url = "https://api.btcmap.org/rpc";
        let method = "POST";
        let b64 = make_nip98_event(&keys, url, method).await;
        let result = verify(&b64, url, method);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().pubkey, keys.public_key().to_hex());
    }

    #[actix_web::test]
    async fn wrong_url() {
        let keys = Keys::generate();
        let b64 = make_nip98_event(&keys, "https://example.com/wrong", "POST").await;
        let result = verify(&b64, "https://api.btcmap.org/rpc", "POST");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("URL mismatch"));
    }

    #[actix_web::test]
    async fn wrong_method() {
        let keys = Keys::generate();
        let url = "https://api.btcmap.org/rpc";
        let b64 = make_nip98_event(&keys, url, "GET").await;
        let result = verify(&b64, url, "POST");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Method mismatch"));
    }

    #[actix_web::test]
    async fn expired_event() {
        let keys = Keys::generate();
        let url = "https://api.btcmap.org/rpc";
        let event = EventBuilder::new(Kind::from_u16(NIP98_KIND), "")
            .tags(vec![
                Tag::parse(["u", url]).unwrap(),
                Tag::parse(["method", "POST"]).unwrap(),
            ])
            .custom_created_at(Timestamp::from(0))
            .sign(&keys)
            .await
            .unwrap();
        let b64 = BASE64.encode(event.as_json().as_bytes());
        let result = verify(&b64, url, "POST");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("timestamp too far"));
    }

    #[actix_web::test]
    async fn wrong_kind() {
        let keys = Keys::generate();
        let url = "https://api.btcmap.org/rpc";
        let event = EventBuilder::new(Kind::from_u16(1), "")
            .tags(vec![
                Tag::parse(["u", url]).unwrap(),
                Tag::parse(["method", "POST"]).unwrap(),
            ])
            .custom_created_at(Timestamp::now())
            .sign(&keys)
            .await
            .unwrap();
        let b64 = BASE64.encode(event.as_json().as_bytes());
        let result = verify(&b64, url, "POST");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid event kind"));
    }

    #[test]
    fn invalid_base64() {
        let result = verify("not-valid-base64!!!", "https://example.com", "POST");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid base64"));
    }

    #[test]
    fn extract_nostr_auth_valid() {
        let header = "Nostr eyJpZCI6ImZlOTY0ZTc1ODkwMzM2MGYyOGQ4NDI0ZDA5MmRh";
        let result = extract_nostr_auth(header);
        assert_eq!(
            result,
            Some("eyJpZCI6ImZlOTY0ZTc1ODkwMzM2MGYyOGQ4NDI0ZDA5MmRh")
        );
    }

    #[test]
    fn extract_nostr_auth_bearer() {
        let result = extract_nostr_auth("Bearer some-token");
        assert_eq!(result, None);
    }
}
