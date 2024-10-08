use super::model::RpcArea;
use crate::{
    admin,
    area::{self},
    discord, Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

const NAME: &str = "set_area_tag";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
    pub name: String,
    pub value: Value,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<RpcArea> {
    let admin = admin::service::check_rpc(&args.password, NAME, &pool).await?;
    let cloned_name = args.name.clone();
    let cloned_value = args.value.clone();
    let area = pool
        .get()
        .await?
        .interact(move |conn| area::service::patch_tag(&args.id, &cloned_name, &cloned_value, conn))
        .await??;
    let log_message = format!(
        "{} set tag {} = {} for area {} https://api.btcmap.org/v3/areas/{}",
        admin.name,
        args.name,
        serde_json::to_string(&args.value)?,
        area.name(),
        area.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area.into())
}
