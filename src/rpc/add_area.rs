use super::model::RpcArea;
use crate::admin::Admin;
use crate::Result;
use crate::{area, discord};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::sync::Arc;
use tracing::info;

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub tags: Map<String, Value>,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<RpcArea> {
    let admin = pool
        .get()
        .await?
        .interact(move |conn| Admin::select_by_password(&args.password, conn))
        .await??
        .unwrap();
    let area = pool
        .get()
        .await?
        .interact(move |conn| area::service::insert(args.tags, conn))
        .await??;
    let log_message = format!(
        "{} created area {} https://api.btcmap.org/v3/areas/{}",
        admin.name,
        area.name(),
        area.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area.into())
}
