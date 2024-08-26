use super::Area;
use crate::{area, auth::Token, discord, Error};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::sync::Arc;
use tracing::info;

#[derive(Deserialize)]
pub struct CreateAreaArgs {
    pub token: String,
    pub tags: Map<String, Value>,
}

pub async fn create(
    Params(args): Params<CreateAreaArgs>,
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
pub struct GetAreaArgs {
    pub token: String,
    pub area_id_or_alias: String,
}

pub async fn get(Params(args): Params<GetAreaArgs>, pool: Data<Arc<Pool>>) -> Result<Area, Error> {
    pool.get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let area = pool
        .get()
        .await?
        .interact(move |conn| Area::select_by_id_or_alias(&args.area_id_or_alias, conn))
        .await??
        .unwrap();
    Ok(area)
}

#[derive(Deserialize)]
pub struct SetAreaTagArgs {
    pub token: String,
    pub area_id_or_alias: String,
    pub tag_name: String,
    pub tag_value: Value,
}

pub async fn set_tag(
    Params(params): Params<SetAreaTagArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Area, Error> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&params.token, conn))
        .await??
        .unwrap();
    let cloned_tag_name = params.tag_name.clone();
    let cloned_tag_value = params.tag_value.clone();
    let area = pool
        .get()
        .await?
        .interact(move |conn| {
            area::service::patch_tag(
                &params.area_id_or_alias,
                &cloned_tag_name,
                &cloned_tag_value,
                conn,
            )
        })
        .await??;
    let log_message = format!(
        "{} set tag {} = {} for area {} https://api.btcmap.org/v3/areas/{}",
        token.owner,
        params.tag_name,
        serde_json::to_string(&params.tag_value)?,
        area.name(),
        area.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area)
}

#[derive(Deserialize)]
pub struct RemoveAreaTagArgs {
    pub token: String,
    pub area_id_or_alias: String,
    pub tag_name: String,
}

pub async fn remove_tag(
    Params(params): Params<RemoveAreaTagArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Area, Error> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&params.token, conn))
        .await??
        .unwrap();
    let cloned_tag_name = params.tag_name.clone();
    let area = pool
        .get()
        .await?
        .interact(move |conn| {
            area::service::remove_tag(&params.area_id_or_alias, &cloned_tag_name, conn)
        })
        .await??;
    let log_message = format!(
        "{} removed tag {} from area {} https://api.btcmap.org/v3/areas/{}",
        token.owner,
        params.tag_name,
        area.name(),
        area.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area)
}

#[derive(Deserialize)]
pub struct RemoveAreaArgs {
    pub token: String,
    pub area_id: i64,
}

pub async fn remove(
    Params(params): Params<RemoveAreaArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Area, Error> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&params.token, conn))
        .await??
        .unwrap();
    let area = pool
        .get()
        .await?
        .interact(move |conn| area::service::soft_delete(params.area_id, conn))
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
