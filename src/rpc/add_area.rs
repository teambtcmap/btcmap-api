use crate::{area, discord, Error};
use crate::{area::Area, auth::Token};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::sync::Arc;
use tracing::info;

#[derive(Deserialize)]
pub struct Args {
    pub token: String,
    pub tags: Map<String, Value>,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Area, Error> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let area = pool
        .get()
        .await?
        .interact(move |conn| area::service::insert(args.tags, conn))
        .await??;
    let log_message = format!(
        "{} created area {} https://api.btcmap.org/v3/areas/{}",
        token.owner,
        area.name(),
        area.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area)
}
