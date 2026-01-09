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
}

impl From<Event> for Res {
    fn from(event: Event) -> Self {
        Res { id: event.id }
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    db::event::queries::set_deleted_at(params.id, Some(OffsetDateTime::now_utc()), pool)
        .await
        .map(Into::into)
}
