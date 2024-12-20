use crate::osm::overpass;
use crate::{admin, sync::MergeResult};
use crate::{db, discord, sync, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

const NAME: &str = "sync_elements";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<MergeResult> {
    let admin = admin::service::check_rpc(args.password, NAME, &pool).await?;
    let elements = overpass::query_bitcoin_merchants().await?;
    let mut conn = db::open_connection()?;
    let res = sync::merge_overpass_elements(elements, &mut conn).await?;
    if res.elements_created + res.elements_updated + res.elements_deleted > 3 {
        let log_message = format!(
            "{} ran a sync with high number of changes (created: {}, updated: {}, deleted: {})",
            admin.name, res.elements_created, res.elements_updated, res.elements_deleted,
        );
        info!(log_message);
        discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    }
    Ok(res)
}
