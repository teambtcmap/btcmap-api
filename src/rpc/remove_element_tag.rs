use crate::discord;
use crate::Result;
use crate::{admin::Admin, element::model::Element};
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

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Element> {
    let admin = pool
        .get()
        .await?
        .interact(move |conn| Admin::select_by_password(&args.password, conn))
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
        admin.name,
        args.tag,
        element.name(),
        element.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(element)
}
