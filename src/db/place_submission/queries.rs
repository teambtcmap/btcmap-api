use crate::{
    db::place_submission::{blocking_queries, schema::PlaceSubmission},
    Result,
};
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use time::OffsetDateTime;

pub async fn insert(
    origin: String,
    external_id: String,
    lat: f64,
    lon: f64,
    category: String,
    name: String,
    extra_fields: JsonObject,
    pool: &Pool,
) -> Result<PlaceSubmission> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::insert(
                &origin,
                &external_id,
                lat,
                lon,
                &category,
                &name,
                &extra_fields,
                conn,
            )
        })
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

pub async fn select_pending_by_bbox(
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
    pool: &Pool,
) -> Result<Vec<PlaceSubmission>> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::select_pending_by_bbox(min_lat, max_lat, min_lon, max_lon, conn)
        })
        .await?
}

pub async fn select_by_search_query(
    search_query: impl Into<String>,
    include_deleted_and_closed: bool,
    pool: &Pool,
) -> Result<Vec<PlaceSubmission>> {
    let search_query = search_query.into();
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::select_by_search_query(search_query, include_deleted_and_closed, conn)
        })
        .await?
}

pub async fn select_by_origin(origin: String, pool: &Pool) -> Result<Vec<PlaceSubmission>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_origin(&origin, conn))
        .await?
}

pub async fn select_updated_since(
    updated_since: OffsetDateTime,
    limit: Option<i64>,
    include_deleted_and_closed: bool,
    pool: &Pool,
) -> Result<Vec<PlaceSubmission>> {
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::select_updated_since(
                updated_since,
                limit,
                include_deleted_and_closed,
                conn,
            )
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
