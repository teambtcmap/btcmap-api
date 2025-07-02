use super::{queries, schema::Event};
use crate::Result;
use deadpool_sqlite::Pool;
use serde_json::Value;
use std::collections::HashMap;

pub async fn insert(
    user_id: i64,
    element_id: i64,
    r#type: impl Into<String>,
    pool: &Pool,
) -> Result<Event> {
    let r#type = r#type.into();
    pool.get()
        .await?
        .interact(move |conn| queries::insert(user_id, element_id, &r#type, conn))
        .await?
}

pub async fn select_all(
    sort_order: Option<String>,
    limit: Option<i64>,
    pool: &Pool,
) -> Result<Vec<Event>> {
    pool.get()
        .await?
        .interact(move |conn| queries::select_all(sort_order, limit, conn))
        .await?
}

pub async fn patch_tags(id: i64, tags: HashMap<String, Value>, pool: &Pool) -> Result<Event> {
    pool.get()
        .await?
        .interact(move |conn| queries::patch_tags(id, &tags, conn))
        .await?
}
