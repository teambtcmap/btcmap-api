use crate::{
    db::{self, event::schema::Event},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

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
    let event = db::event::queries::select_by_id(params.id, pool).await?;
    Ok(event.into())
}
