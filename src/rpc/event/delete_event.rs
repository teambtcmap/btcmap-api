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
    id: i64,
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
    let event =
        db::event::queries::set_deleted_at(params.id, Some(OffsetDateTime::now_utc()), pool)
            .await?;
    discord::send(
        format!(
            "Deleted event (id: {}, name: {}, date: {})",
            event.id,
            event.name,
            event.starts_at.format(&Rfc3339)?,
        ),
        discord::Channel::Api,
        conf,
    );
    Ok(event.into())
}
