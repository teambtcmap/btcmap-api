use crate::{
    db::{self},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Deserialize, Clone)]
pub struct Params {
    pub user_name: String,
    pub tag_name: String,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub tags: Map<String, Value>,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let user = db::osm_user::queries::select_by_name(params.user_name.clone(), pool).await?;
    let user = db::osm_user::queries::remove_tag(user.id, params.tag_name.clone(), pool).await?;
    Ok(Res {
        id: user.id,
        tags: user.tags,
    })
}
