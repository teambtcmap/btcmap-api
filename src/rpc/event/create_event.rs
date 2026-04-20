use crate::{
    db::{self, main::event::schema::Event},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    area_id: Option<i64>,
    lat: f64,
    lon: f64,
    name: String,
    website: String,
    #[serde(with = "time::serde::rfc3339::option")]
    starts_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    ends_at: Option<OffsetDateTime>,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
}

impl From<Event> for Res {
    fn from(event: Event) -> Self {
        Res { id: event.id }
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    db::main::event::queries::insert(
        params.area_id,
        params.lat,
        params.lon,
        params.name,
        params.website,
        params.starts_at,
        params.ends_at,
        pool,
    )
    .await
    .map(Into::into)
}
