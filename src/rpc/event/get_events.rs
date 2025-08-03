use crate::{
    db::{self, event::schema::Event},
    Result,
};
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub lat: f64,
    pub lon: f64,
    pub name: String,
    pub website: String,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
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

pub async fn run(pool: &Pool) -> Result<Vec<Res>> {
    let events = db::event::queries::select_all(pool).await?;
    Ok(events.into_iter().map(Into::into).collect())
}
