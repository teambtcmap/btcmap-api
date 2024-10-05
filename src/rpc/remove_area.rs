use super::model::RpcArea;
use crate::Result;
use crate::{admin, area, discord};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

const NAME: &str = "remove_area";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<RpcArea> {
    let admin = admin::service::check_rpc(&args.password, NAME, &pool).await?;
    let area = pool
        .get()
        .await?
        .interact(move |conn| area::service::soft_delete(&args.id, conn))
        .await??;
    let log_message = format!(
        "{} removed area {} https://api.btcmap.org/v3/areas/{}",
        admin.name,
        area.name(),
        area.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area.into())
}
