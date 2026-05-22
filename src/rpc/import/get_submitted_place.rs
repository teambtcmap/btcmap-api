use crate::{
    db::main::{access_token::schema::AccessToken, user::schema::Role},
    db::{self},
    Result,
};
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub id: Option<i64>,
    pub origin: Option<String>,
    pub external_id: Option<String>,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub origin: String,
    pub external_id: String,
    pub lat: f64,
    pub lon: f64,
    pub category: String,
    pub name: String,
    pub extra_fields: JsonObject,
    pub revoked: bool,
}

pub async fn run(params: Params, roles: &[Role], token: &AccessToken, pool: &Pool) -> Result<Res> {
    let submission = match params.id {
        Some(id) => db::main::place_submission::queries::select_by_id(id, pool).await?,
        None => {
            let Some(origin) = params.origin else {
                return Err("missing parameter: origin or id".into());
            };
            let Some(external_id) = params.external_id else {
                return Err("missing parameter: external_id or id".into());
            };
            db::main::place_submission::queries::select_by_origin_and_external_id(
                origin,
                external_id,
                pool,
            )
            .await?
            .ok_or("can't find place with provided origin and external_id")?
        }
    };

    super::ensure_can_access_origin(roles, token, &submission.origin)?;

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
