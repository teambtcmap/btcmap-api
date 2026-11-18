use serde::Deserialize;

pub mod activity;
pub mod area_reports;
pub mod areas;
pub mod auth;
pub mod communities;
pub mod countries;
pub mod dashboard;
pub mod events;
pub mod invoices;
pub mod nostr;
pub mod notes;
pub mod opengraph;
pub mod place_boosts;
pub mod place_comments;
pub mod place_images;
pub mod place_issues;
pub mod place_reports;
pub mod place_submissions;
pub mod places;
pub mod search;
pub mod top_editors;
pub mod users;

/// Tri-state JSON field for `PATCH` bodies: absent (`None`), explicit `null`
/// (`Some(None)`) or a value (`Some(Some(_))`). A plain `Option<Option<T>>`
/// would need a `serde(deserialize_with)` helper, which `ts-rs` cannot parse;
/// implementing `Deserialize` here keeps the exported binding warning-free.
pub struct Nullable<T>(Option<Option<T>>);

impl<T> Default for Nullable<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<'de, T> Deserialize<'de> for Nullable<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Nullable(Some(Option::<T>::deserialize(deserializer)?)))
    }
}
