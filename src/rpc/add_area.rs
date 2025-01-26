use super::model::RpcArea;
use crate::admin;
use crate::conf::Conf;
use crate::Result;
use crate::{area, discord};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::sync::Arc;
use tracing::info;

const NAME: &str = "add_area";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub tags: Map<String, Value>,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<RpcArea> {
    let admin = admin::service::check_rpc(args.password, NAME, &pool).await?;
    let area = pool
        .get()
        .await?
        .interact(move |conn| area::service::insert(args.tags, conn))
        .await??;
    let log_message = format!(
        "Admin {} created area {} https://api.btcmap.org/v3/areas/{}",
        admin.name,
        area.name(),
        area.id,
    );
    info!(log_message);
    let conf = Conf::select_async(&pool).await?;
    discord::post_message(conf.discord_webhook_api, log_message).await;
    Ok(area.into())
}
