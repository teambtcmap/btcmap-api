use crate::admin;
use crate::boost::Boost;
use crate::Result;
use deadpool_sqlite::Pool;
use jsonrpc_v2::Data;
use serde::Deserialize;
use std::sync::Arc;

pub const NAME: &str = "get_boosted_elements";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
}

pub async fn run(
    jsonrpc_v2::Params(params): jsonrpc_v2::Params<Params>,
    pool: Data<Arc<Pool>>,
) -> Result<Vec<Boost>> {
    run_internal(params, &pool).await
}

pub async fn run_internal(params: Params, pool: &Pool) -> Result<Vec<Boost>> {
    admin::service::check_rpc(params.password, NAME, &pool).await?;
    let boosts = pool
        .get()
        .await?
        .interact(move |conn| Boost::select_all(conn))
        .await??;
    Ok(boosts)
}
