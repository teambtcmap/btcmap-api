use crate::Result;
use crate::{admin::Admin, element::model::Element};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Element> {
    pool.get()
        .await?
        .interact(move |conn| Admin::select_by_password(&args.password, conn))
        .await??
        .unwrap();
    let element = pool
        .get()
        .await?
        .interact(move |conn| Element::select_by_id_or_osm_id(&args.id, conn))
        .await??
        .unwrap();
    Ok(element)
}
