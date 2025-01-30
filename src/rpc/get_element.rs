use crate::element::model::Element;
use crate::{element, Result};
use deadpool_sqlite::Pool;
use serde::Deserialize;
use std::sync::Arc;

pub const NAME: &str = "get_element";

#[derive(Deserialize)]
pub struct Params {
    pub id: String,
}

pub async fn run(
    jsonrpc_v2::Params(args): jsonrpc_v2::Params<Params>,
    pool: jsonrpc_v2::Data<Arc<Pool>>,
) -> Result<element::v4::GetItem> {
    run_internal(args, &pool).await
}

pub async fn run_internal(params: Params, pool: &Pool) -> Result<element::v4::GetItem> {
    Element::select_by_id_or_osm_id_async(&params.id, pool)
        .await?
        .map(Into::into)
        .ok_or(format!("There is no element with id {}", params.id).into())
}
