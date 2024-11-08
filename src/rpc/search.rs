use crate::{admin, area::Area, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const NAME: &str = "search";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub query: String,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub r#type: String,
    pub id: i64,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Vec<Res>> {
    admin::service::check_rpc(&args.password, NAME, &pool).await?;
    let areas = pool
        .get()
        .await?
        .interact(move |conn| Area::select_by_search_query(&args.query, conn))
        .await??;
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
