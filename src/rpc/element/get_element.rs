use crate::{db, service, Result};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Deserialize)]
pub struct Params {
    pub id: i64,
}

#[derive(Serialize)]
pub struct Res {
    element: Map<String, Value>,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let element = db::element::queries_async::select_by_id(params.id, pool).await?;
    Ok(Res {
        element: service::element::generate_tags(&element, service::element::TAGS),
    })
}
