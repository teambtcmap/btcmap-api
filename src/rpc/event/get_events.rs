use crate::{
    db::{self, main::event::schema::Event},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub lat: f64,
    pub lon: f64,
    pub name: String,
    pub website: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub starts_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
    pub cron_schedule: Option<String>,
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
        }
    }
}

#[derive(Deserialize)]
pub struct Params {
    include_past: Option<bool>,
    include_deleted: Option<bool>,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Vec<Res>> {
    let events = db::main::event::queries::select_all(pool).await?;
    let include_past = params.include_past.unwrap_or(false);
    let include_deleted = params.include_deleted.unwrap_or(false);
    let now = OffsetDateTime::now_utc();
    let events: Vec<Event> = events
        .into_iter()
        .filter(|it| {
            (include_deleted || it.deleted_at.is_none())
                && (include_past || it.starts_at.is_none() || it.starts_at > Some(now))
        })
        .collect();
    Ok(events.into_iter().map(Into::into).collect())
}
