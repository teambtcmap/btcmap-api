use super::model::RpcArea;
use crate::{area::Area, Result};
use deadpool_sqlite::Pool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub id: String,
}

pub async fn run(params: Params, pool: &Pool) -> Result<RpcArea> {
    Area::select_by_id_or_alias_async(params.id, &pool)
        .await
        .map(Into::into)
}
