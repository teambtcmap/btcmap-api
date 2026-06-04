use crate::{
    db::main::place_submission::{
        blocking_queries, blocking_queries::InsertArgs, schema::PlaceSubmission,
    },
    Result,
};
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use time::OffsetDateTime;

pub async fn insert(args: InsertArgs, pool: &Pool) -> Result<PlaceSubmission> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::insert(&args, conn))
        .await?
}

pub async fn select_open_and_not_revoked(pool: &Pool) -> Result<Vec<PlaceSubmission>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_open_and_not_revoked(conn))
        .await?
}

pub async fn select_by_id(id: i64, pool: &Pool) -> Result<PlaceSubmission> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_id(id, conn))
        .await?
}

pub async fn select_by_origin_and_external_id(
    origin: String,
    external_id: String,
    pool: &Pool,
) -> Result<Option<PlaceSubmission>> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::select_by_origin_and_external_id(&origin, &external_id, conn)
        })
        .await?
}

pub async fn set_fields(
    id: i64,
    lat: f64,
    lon: f64,
    category: String,
    name: String,
    extra_fields: &JsonObject,
    pool: &Pool,
) -> Result<PlaceSubmission> {
    let extra_fields = extra_fields.clone();
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::set_fields(id, lat, lon, &category, &name, extra_fields, conn)
        })
        .await?
}

pub async fn set_revoked(id: i64, revoked: bool, pool: &Pool) -> Result<PlaceSubmission> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_revoked(id, revoked, conn))
        .await?
}

pub async fn set_ticket_url(id: i64, ticket_url: String, pool: &Pool) -> Result<PlaceSubmission> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_ticket_url(id, ticket_url, conn))
        .await?
}

pub async fn set_closed_at(
    id: i64,
    closed_at: Option<OffsetDateTime>,
    pool: &Pool,
) -> Result<PlaceSubmission> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_closed_at(id, closed_at, conn))
        .await?
}
