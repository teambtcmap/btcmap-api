use nostr::event::{Event, Kind};
use nostr::key::PublicKey;
use nostr::nips::nip01::Metadata;

/// Longest username taken from a profile, in characters (matches the web's
/// username field).
pub const PROFILE_NAME_MAX_CHARS: usize = 100;

/// The username a signed kind-0 profile event asks for: its `name`, else
/// its `display_name`, cleaned up (see [`clean_name`]). None unless the
/// event is a valid kind 0 signed by `signer`, so a client can't pick a
/// name for someone else's profile.
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
        .map(|name| clean_name(&name))
        .find(|name| !name.is_empty())
}

/// `name` as a username: runs of whitespace (line breaks included) become
/// one space, control characters and invisible formatting (bidi overrides,
/// zero-width spaces, ...) are dropped, and the result is capped. The
/// zero-width joiner and non-joiner stay: emoji sequences and some scripts
/// need them.
fn clean_name(name: &str) -> String {
    let visible: String = name
        .chars()
        .filter(|c| c.is_whitespace() || !(c.is_control() || is_invisible_format(*c)))
        .collect();
    visible
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(PROFILE_NAME_MAX_CHARS)
        .collect()
}

/// Unicode format characters that render as nothing or reorder text,
/// except the joiners (U+200C, U+200D).
fn is_invisible_format(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{061C}'
            | '\u{180E}'
            | '\u{200B}'
            | '\u{200E}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
    )
}

#[cfg(test)]
mod test {
    use super::clean_name;

    #[test]
    fn clean_name_joins_lines_and_drops_control_characters() {
        assert_eq!(
            clean_name("  Satoshi\n\tNakamoto\u{7} "),
            "Satoshi Nakamoto"
        );
    }

    #[test]
    fn clean_name_drops_invisible_formatting() {
        assert_eq!(clean_name("\u{202E}sa\u{200B}toshi\u{FEFF}"), "satoshi");
    }

    #[test]
    fn clean_name_keeps_joiners_inside_emoji_and_scripts() {
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        assert_eq!(clean_name(family), family);
        let persian = "\u{0645}\u{06CC}\u{200C}\u{062E}\u{0648}\u{0627}\u{0647}\u{0645}";
        assert_eq!(clean_name(persian), persian);
    }

    #[test]
    fn clean_name_caps_the_length() {
        assert_eq!(clean_name(&"a".repeat(150)).chars().count(), 100);
    }
}
