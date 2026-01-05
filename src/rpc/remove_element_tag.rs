use crate::db;
use crate::Result;
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub element_id: i64,
    pub tag_name: String,
}

#[derive(Serialize)]
pub struct Res {
    id: i64,
    tags: JsonObject,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let element = db::element::queries::select_by_id(params.element_id, pool).await?;
    let element = db::element::queries::remove_tag(element.id, &params.tag_name, pool).await?;
    Ok(Res {
        id: element.id,
        tags: element.tags,
    })
}
