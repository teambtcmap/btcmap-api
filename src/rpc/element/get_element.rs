use crate::element::{self, model::Element};
use crate::Result;
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
    let element = Element::select_by_id_async(params.id, pool).await?;
    Ok(Res {
        element: crate::element::service::generate_tags(&element, element::service::TAGS),
    })
}
