use crate::{
    db::{self, event::schema::Event},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    id: i64,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    lat: f64,
    lon: f64,
    name: String,
    website: String,
    #[serde(with = "time::serde::rfc3339::option")]
    starts_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    ends_at: Option<OffsetDateTime>,
}

impl From<Event> for Res {
    fn from(event: Event) -> Self {
        Res {
            id: event.id,
            lat: event.lat,
            lon: event.lon,
            name: event.name,
            website: event.website,
            starts_at: event.starts_at,
            ends_at: event.ends_at,
        }
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let event = db::event::queries::select_by_id(params.id, pool).await?;
    Ok(event.into())
}
