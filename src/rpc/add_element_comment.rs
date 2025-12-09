use crate::{
    db::{self},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub element_id: i64,
    pub comment: String,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let element = db::element::queries::select_by_id(params.element_id, pool).await?;
    let comment = db::element_comment::queries::insert(element.id, &params.comment, pool).await?;
    Ok(Res { id: comment.id })
}
