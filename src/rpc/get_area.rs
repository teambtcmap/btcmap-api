use super::model::RpcArea;
use crate::{admin, area::Area, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;

const NAME: &str = "get_area";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<RpcArea> {
    admin::service::check_rpc(args.password, NAME, &pool).await?;
    Area::select_by_id_or_alias_async(&args.id, &pool)
        .await
        .map(Into::into)
}
