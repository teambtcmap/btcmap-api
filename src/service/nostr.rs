use nostr::event::{Event, Kind};
use nostr::key::PublicKey;
use nostr::nips::nip01::Metadata;

/// Longest username taken from a profile, in characters (matches the web's
/// username field).
const PROFILE_NAME_MAX_CHARS: usize = 100;

/// The username a signed kind-0 profile event asks for: its `name`, else
/// its `display_name`, trimmed and capped. None unless the event is a valid
/// kind 0 signed by `signer`, so a client can't pick a name for
/// someone else's profile.
pub fn profile_name(profile: &serde_json::Value, signer: &PublicKey) -> Option<String> {
    let event = Event::from_json(profile.to_string()).ok()?;
    event.verify().ok()?;
    if event.kind != Kind::Metadata || event.pubkey != *signer {
        return None;
    }
    let metadata = Metadata::from_json(&event.content).ok()?;
    [metadata.name, metadata.display_name]
        .into_iter()
        .flatten()
        .map(|name| {
            name.trim()
                .chars()
                .take(PROFILE_NAME_MAX_CHARS)
                .collect::<String>()
        })
        .find(|name| !name.is_empty())
}
