use std::sync::Arc;

use crate::Result;
use crate::{admin, element::model::Element};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;

const NAME: &str = "get_element";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Element> {
    admin::service::check_rpc(&args.password, NAME, &pool).await?;
    let cloned_args_id = args.id.clone();
    let element = pool
        .get()
        .await?
        .interact(move |conn| Element::select_by_id_or_osm_id(&cloned_args_id, conn))
        .await??
        .ok_or(format!(
            "There is no element with id or osm_id = {}",
            args.id
        ))?;
    Ok(element)
}
