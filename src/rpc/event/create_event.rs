use crate::{
    db::{self, conf::schema::Conf, event::schema::Event},
    service::discord,
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Deserialize)]
pub struct Params {
    lat: f64,
    lon: f64,
    name: String,
    website: String,
    #[serde(with = "time::serde::rfc3339")]
    starts_at: OffsetDateTime,
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

pub async fn run(params: Params, pool: &Pool, conf: &Conf) -> Result<Res> {
    let event = db::event::queries::insert(
        params.lat,
        params.lon,
        params.name,
        params.website,
        params.starts_at,
        params.ends_at,
        pool,
    )
    .await?;
    discord::send(
        format!(
            "New event (id: {}, name: {}, date: {})",
            event.id,
            event.name,
            event.starts_at.format(&Rfc3339)?,
        ),
        discord::Channel::Api,
        conf,
    );
    Ok(event.into())
}
