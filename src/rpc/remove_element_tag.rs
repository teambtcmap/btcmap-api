use crate::discord;
use crate::Result;
use crate::{admin, element::model::Element};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

const NAME: &str = "remove_element_tag";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
    pub tag: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Element> {
    let admin = admin::service::check_rpc(args.password, NAME, &pool).await?;
    let cloned_args_id = args.id.clone();
    let element = pool
        .get()
        .await?
        .interact(move |conn| Element::select_by_id_or_osm_id(&cloned_args_id, conn))
        .await??
        .ok_or(format!(
            "There is no element with id or osm_id = {}",
            args.id,
        ))?;
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
