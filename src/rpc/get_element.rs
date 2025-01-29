use crate::Result;
use crate::{admin, element::model::Element};
use deadpool_sqlite::Pool;
use serde::Deserialize;
use std::sync::Arc;

pub const NAME: &str = "get_element";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
    pub id: String,
}

pub async fn run(
    jsonrpc_v2::Params(args): jsonrpc_v2::Params<Params>,
    pool: jsonrpc_v2::Data<Arc<Pool>>,
) -> Result<Element> {
    run_internal(args, &pool).await
}

pub async fn run_internal(params: Params, pool: &Pool) -> Result<Element> {
    admin::service::check_rpc(params.password, NAME, &pool).await?;
    Element::select_by_id_or_osm_id_async(&params.id, pool)
        .await?
        .ok_or(format!("There is no element with id or osm_id = {}", params.id).into())
}
