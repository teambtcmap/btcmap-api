use super::model::RpcArea;
use crate::{
    admin,
    area::{self},
    discord, Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use tracing::info;

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
    pub tag: String,
}

const NAME: &str = "remove_area_tag";

pub async fn run(Params(args): Params<Args>, pool: Data<Pool>) -> Result<RpcArea> {
    let admin = admin::service::check_rpc(&args.password, NAME, &pool).await?;
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
