use crate::{
    db::place_submission::{blocking_queries, schema::PlaceSubmission},
    Result,
};
use deadpool_sqlite::Pool;
use geojson::JsonObject;

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

// pub async fn set_closed_at(
//     id: i64,
//     closed_at: Option<OffsetDateTime>,
//     pool: &Pool,
// ) -> Result<PlaceSubmission> {
//     pool.get()
//         .await?
//         .interact(move |conn| blocking_queries::set_closed_at(id, closed_at, conn))
//         .await?
// }
