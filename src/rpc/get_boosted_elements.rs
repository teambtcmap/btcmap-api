use crate::admin;
use crate::boost::Boost;
use crate::Result;
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;

const NAME: &str = "get_boosted_elements";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Vec<Boost>> {
    admin::service::check_rpc(args.password, NAME, &pool).await?;
    let boosts = pool
        .get()
        .await?
        .interact(move |conn| Boost::select_all(conn))
        .await??;
    Ok(boosts)
}
