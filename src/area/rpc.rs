use super::Area;
use crate::{area, auth::Token, discord, Error};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::sync::Arc;
use tracing::info;

#[derive(Deserialize)]
pub struct CreateArgs {
    pub token: String,
    pub tags: Map<String, Value>,
}

pub async fn create(
    Params(args): Params<CreateArgs>,
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

#[derive(Deserialize)]
pub struct GetArgs {
    pub token: String,
    pub id: String,
}

pub async fn get(Params(args): Params<GetArgs>, pool: Data<Arc<Pool>>) -> Result<Area, Error> {
    pool.get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let area = pool
        .get()
        .await?
        .interact(move |conn| Area::select_by_id_or_alias(&args.id, conn))
        .await??
        .unwrap();
    Ok(area)
}

#[derive(Deserialize)]
pub struct SetTagArgs {
    pub token: String,
    pub id: String,
    pub name: String,
    pub value: Value,
}

pub async fn set_tag(
    Params(args): Params<SetTagArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Area, Error> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let cloned_name = args.name.clone();
    let cloned_value = args.value.clone();
    let area = pool
        .get()
        .await?
        .interact(move |conn| area::service::patch_tag(&args.id, &cloned_name, &cloned_value, conn))
        .await??;
    let log_message = format!(
        "{} set tag {} = {} for area {} https://api.btcmap.org/v3/areas/{}",
        token.owner,
        args.name,
        serde_json::to_string(&args.value)?,
        area.name(),
        area.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area)
}

#[derive(Deserialize)]
pub struct RemoveTagArgs {
    pub token: String,
    pub id: String,
    pub tag: String,
}

pub async fn remove_tag(
    Params(args): Params<RemoveTagArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Area, Error> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
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
        token.owner,
        args.tag,
        area.name(),
        area.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area)
}

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
