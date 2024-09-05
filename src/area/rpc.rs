use super::Area;
use crate::Result;
use crate::{area, auth::Token, discord, Error};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

#[derive(Deserialize)]
pub struct RemoveArgs {
    pub token: String,
    pub id: String,
}

pub async fn remove(
    Params(args): Params<RemoveArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Area, Error> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let area = pool
        .get()
        .await?
        .interact(move |conn| area::service::soft_delete(&args.id, conn))
        .await??;
    let log_message = format!(
        "{} removed area {} https://api.btcmap.org/v3/areas/{}",
        token.owner,
        area.name(),
        area.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area)
}
