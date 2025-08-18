use super::model::RpcArea;
use crate::{db, Result};
use deadpool_sqlite::Pool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub id: String,
}

pub async fn run(params: Params, pool: &Pool) -> Result<RpcArea> {
    db::area::queries::select_by_id_or_alias(params.id, pool)
        .await
        .map(Into::into)
}
