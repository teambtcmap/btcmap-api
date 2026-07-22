use crate::{
    db::{self, main::event::schema::Event},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

mod optional_rfc3339 {
    use serde::{Deserialize, Deserializer};
    use time::OffsetDateTime;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Option<OffsetDateTime>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Some(Option::deserialize(deserializer)?))
    }
}

#[derive(Deserialize)]
pub struct Params {
    pub id: i64,
    #[serde(default)]
    pub area_id: Option<Option<i64>>,
    #[serde(default)]
    lat: Option<f64>,
    #[serde(default)]
    lon: Option<f64>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    website: Option<String>,
    #[serde(default, deserialize_with = "optional_rfc3339::deserialize")]
    starts_at: Option<Option<OffsetDateTime>>,
    #[serde(default, deserialize_with = "optional_rfc3339::deserialize")]
    ends_at: Option<Option<OffsetDateTime>>,
    #[serde(default)]
    cron_schedule: Option<Option<String>>,
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
    cron_schedule: Option<String>,
    pub area_id: Option<i64>,
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
            cron_schedule: event.cron_schedule,
            area_id: event.area_id,
        }
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    db::main::event::queries::update(
        params.id,
        params.area_id,
        params.lat,
        params.lon,
        params.name,
        params.website,
        params.starts_at,
        params.ends_at,
        params.cron_schedule,
        pool,
    )
    .await
    .map(Into::into)
}
