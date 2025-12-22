use super::blocking_queries;
use crate::{db::rpc_call::schema::RpcCall, Result};
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use time::OffsetDateTime;

pub async fn insert(
    user_id: i64,
    ip: String,
    method: String,
    params: Option<JsonObject>,
    created_at: OffsetDateTime,
    processed_at: OffsetDateTime,
    pool: &Pool,
) -> Result<RpcCall> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::insert(user_id, ip, method, params, created_at, processed_at, conn)
        })
        .await?
}
