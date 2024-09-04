use crate::discord;
use crate::Result;
use crate::{auth::Token, element::model::Element};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

#[derive(Deserialize)]
pub struct GetArgs {
    pub token: String,
    pub id: String,
}

pub async fn get(Params(args): Params<GetArgs>, pool: Data<Arc<Pool>>) -> Result<Element> {
    pool.get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let element = pool
        .get()
        .await?
        .interact(move |conn| Element::select_by_id_or_osm_id(&args.id, conn))
        .await??
        .unwrap();
    Ok(element)
}

#[derive(Deserialize)]
pub struct SetTagArgs {
    pub token: String,
    pub id: String,
    pub name: String,
    pub value: Value,
}

pub async fn set_tag(Params(args): Params<SetTagArgs>, pool: Data<Arc<Pool>>) -> Result<Element> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let element = pool
        .get()
        .await?
        .interact(move |conn| Element::select_by_id_or_osm_id(&args.id, conn))
        .await??
        .unwrap();
    let cloned_name = args.name.clone();
    let cloned_value = args.value.clone();
    let element = pool
        .get()
        .await?
        .interact(move |conn| Element::set_tag(element.id, &cloned_name, &cloned_value, conn))
        .await??;
    let log_message = format!(
        "{} set tag {} = {} for element {} https://api.btcmap.org/v3/elements/{}",
        token.owner,
        args.name,
        serde_json::to_string(&args.value)?,
        element.name(),
        element.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(element)
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
) -> Result<Element> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let element = pool
        .get()
        .await?
        .interact(move |conn| Element::select_by_id_or_osm_id(&args.id, conn))
        .await??
        .unwrap();
    let cloned_tag = args.tag.clone();
    let element = pool
        .get()
        .await?
        .interact(move |conn| Element::remove_tag(element.id, &cloned_tag, conn))
        .await??;
    let log_message = format!(
        "{} removed tag {} from element {} https://api.btcmap.org/v3/elements/{}",
        token.owner,
        args.tag,
        element.name(),
        element.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(element)
}
