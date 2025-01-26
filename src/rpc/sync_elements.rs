use crate::conf::Conf;
use crate::osm::overpass;
use crate::{admin, sync::MergeResult};
use crate::{db, discord, sync, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

const NAME: &str = "sync_elements";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
}

#[derive(Serialize)]
pub struct Res {
    pub overpass_query_time_s: f64,
    pub overpass_elements: usize,
    pub merge_result: MergeResult,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let admin = admin::service::check_rpc(args.password, NAME, &pool).await?;
    info!(admin.name, "Admin requested element sync");
    let overpass_res = overpass::query_bitcoin_merchants().await?;
    let overpass_elements_len = overpass_res.elements.len();
    let mut conn = db::open_connection()?;
    let merge_res = sync::merge_overpass_elements(overpass_res.elements, &mut conn).await?;
    if merge_res.elements_created.len()
        + merge_res.elements_updated.len()
        + merge_res.elements_deleted.len()
        > 3
    {
        let log_message = format!(
            "Admin {} ran a sync with high number of changes (created: {}, updated: {}, deleted: {})",
            admin.name,
            merge_res.elements_created.len(),
            merge_res.elements_updated.len(),
            merge_res.elements_deleted.len(),
        );
        info!(log_message);
        let conf = Conf::select_async(&pool).await?;
        discord::post_message(conf.discord_webhook_api, log_message).await;
    }
    Ok(Res {
        overpass_query_time_s: overpass_res.time_s,
        overpass_elements: overpass_elements_len,
        merge_result: merge_res,
    })
}
