use super::model::RpcArea;
use crate::{area::Area, auth::Token, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct Args {
    pub token: String,
    pub id: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<RpcArea> {
    pool.get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let area = pool
        .get()
        .await?
        .interact(move |conn| Area::select_by_id_or_alias(&args.id, conn))
        .await??
        .unwrap();
    Ok(area.into())
}
