pub const MAX_WORDS: usize = 8;

/// Escapes SQLite `LIKE` metacharacters. Patterns built from this **must** be
/// used with `ESCAPE '\'`, otherwise a query of `%` matches every row.
pub fn escape_like(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

/// Splits a query into at most [`MAX_WORDS`] whitespace-separated words. The cap
/// bounds the number of correlated subqueries a single request can generate.
pub fn split_words(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .take(MAX_WORDS)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod test {
    use super::{escape_like, split_words, MAX_WORDS};

    #[test]
    fn escape_like_escapes_wildcards() {
        assert_eq!("100\\%", escape_like("100%"));
        assert_eq!("a\\_b", escape_like("a_b"));
        assert_eq!("a\\\\b", escape_like("a\\b"));
        assert_eq!("hamburg", escape_like("hamburg"));
    }

    #[test]
    fn escape_like_preserves_non_ascii() {
        assert_eq!("café", escape_like("café"));
    }

    #[test]
    fn split_words_splits_on_whitespace_and_trims() {
        assert_eq!(vec!["hamburg", "cafe"], split_words("  hamburg   cafe "));
    }

    #[test]
    fn split_words_caps_word_count() {
        let query = "a b c d e f g h i j k";
        assert_eq!(MAX_WORDS, split_words(query).len());
    }

    #[test]
    fn split_words_on_empty_query_is_empty() {
        assert!(split_words("   ").is_empty());
    }
}
