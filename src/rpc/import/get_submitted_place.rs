use crate::{
    db::{self},
    Result,
};
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    id: Option<i64>,
    origin: Option<String>,
    external_id: Option<String>,
}

#[derive(Serialize)]
pub struct Res {
    id: i64,
    origin: String,
    external_id: String,
    lat: f64,
    lon: f64,
    category: String,
    name: String,
    extra_fields: JsonObject,
    revoked: bool,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let submission = match params.id {
        Some(id) => db::place_submission::queries::select_by_id(id, pool).await?,
        None => db::place_submission::queries::select_by_origin_and_external_id(
            params.origin.unwrap(),
            params.external_id.unwrap(),
            pool,
        )
        .await?
        .unwrap(),
    };

    Ok(Res {
        id: submission.id,
        origin: submission.origin,
        external_id: submission.external_id,
        lat: submission.lat,
        lon: submission.lon,
        category: submission.category,
        name: submission.name,
        extra_fields: submission.extra_fields,
        revoked: submission.revoked,
    })
}
