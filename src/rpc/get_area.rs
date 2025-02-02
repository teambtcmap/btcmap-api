use super::model::RpcArea;
use crate::{admin, area::Area, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::Data;
use serde::Deserialize;
use std::sync::Arc;

pub const NAME: &str = "get_area";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
    pub id: String,
}

pub async fn run(
    jsonrpc_v2::Params(params): jsonrpc_v2::Params<Params>,
    pool: Data<Arc<Pool>>,
) -> Result<RpcArea> {
    run_internal(params, &pool).await
}

pub async fn run_internal(params: Params, pool: &Pool) -> Result<RpcArea> {
    admin::service::check_rpc(params.password, NAME, &pool).await?;
    Area::select_by_id_or_alias_async(params.id, &pool)
        .await
        .map(Into::into)
}
