use crate::{admin, area::Area, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::Data;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const NAME: &str = "search";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
    pub query: String,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub r#type: String,
    pub id: i64,
}

pub async fn run(
    jsonrpc_v2::Params(params): jsonrpc_v2::Params<Params>,
    pool: Data<Arc<Pool>>,
) -> Result<Vec<Res>> {
    run_internal(params, &pool).await
}

pub async fn run_internal(params: Params, pool: &Pool) -> Result<Vec<Res>> {
    admin::service::check_rpc(params.password, NAME, &pool).await?;
    let areas = Area::select_by_search_query_async(params.query, &pool).await?;
    let res = areas
        .into_iter()
        .map(|it| Res {
            name: it.name(),
            r#type: "area".into(),
            id: it.id,
        })
        .collect();
    Ok(res)
}
