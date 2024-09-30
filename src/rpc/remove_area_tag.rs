use super::model::RpcArea;
use crate::{
    admin::Admin,
    area::{self},
    discord, Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
    pub tag: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<RpcArea> {
    let admin = pool
        .get()
        .await?
        .interact(move |conn| Admin::select_by_password(&args.password, conn))
        .await??
        .unwrap();
    let cloned_tag = args.tag.clone();
    let area = pool
        .get()
        .await?
        .interact(move |conn| area::service::remove_tag(&args.id, &cloned_tag, conn))
        .await??;
    let log_message = format!(
        "{} removed tag {} from area {} https://api.btcmap.org/v3/areas/{}",
        admin.name,
        args.tag,
        area.name(),
        area.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area.into())
}
