use serde_json::{Map, Value};

/// Renders a `Label: value` line.
pub fn field(label: &str, value: &str) -> String {
    format!("{}: {}", label, value)
}

/// Renders a JSON value on a single line, never using JSON syntax: nested
/// objects become `key=value` pairs and arrays a comma-separated list, so a
/// value can't leak braces into an issue body. Whitespace is collapsed so a
/// value can't break the one-field-per-line layout either.
pub fn single_line(value: &Value) -> Option<String> {
    let value = inline(value)?;
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn inline(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) => Some(value.clone()),
        Value::Array(values) => join(values.iter().filter_map(inline)),
        Value::Object(map) => join(
            map.iter()
                .filter_map(|(key, value)| inline(value).map(|value| format!("{}={}", key, value))),
        ),
    }
}

fn join(values: impl Iterator<Item = String>) -> Option<String> {
    let values = values.collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.join(", "))
    }
}

/// First key holding a non-empty value wins, so a caller can accept the
/// different names sources use for the same field.
pub fn extra_value(extra_fields: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| extra_fields.get(*key).and_then(single_line))
}

/// `key: value` lines for every `extra_fields` entry outside `consumed`, so a
/// source can send extra data without it being silently dropped.
pub fn additional_fields(extra_fields: &Map<String, Value>, consumed: &[&str]) -> Vec<String> {
    let mut res = extra_fields
        .iter()
        .filter(|(key, _)| !consumed.contains(&key.as_str()))
        .filter_map(|(key, value)| single_line(value).map(|value| field(key, &value)))
        .collect::<Vec<_>>();
    res.sort();
    res
}

/// `"onchain,lightning"` reads better as `"onchain, lightning"`.
pub fn humanize_list(value: &str) -> String {
    value
        .split([',', ';'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod test {
    use super::single_line;
    use serde_json::json;

    #[test]
    fn nested_values_never_render_json_syntax() {
        assert_eq!(
            single_line(&json!({ "lat": 1.5, "lon": -2.5 })).unwrap(),
            "lat=1.5, lon=-2.5"
        );
        assert_eq!(single_line(&json!(["a", "b"])).unwrap(), "a, b");
        assert_eq!(
            single_line(&json!([{ "a": 1 }, { "b": 2 }])).unwrap(),
            "a=1, b=2"
        );
        assert_eq!(single_line(&json!(7)).unwrap(), "7");
    }

    #[test]
    fn empty_values_are_dropped() {
        assert_eq!(single_line(&json!(null)), None);
        assert_eq!(single_line(&json!("   ")), None);
        assert_eq!(single_line(&json!([])), None);
        assert_eq!(single_line(&json!({})), None);
    }

    #[test]
    fn whitespace_is_collapsed() {
        assert_eq!(single_line(&json!("a\n b\tc")).unwrap(), "a b c");
    }
}
