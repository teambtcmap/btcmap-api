use crate::discord;
use crate::Result;
use crate::{admin, element::model::Element};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

const NAME: &str = "set_element_tag";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
    pub name: String,
    pub value: Value,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Element> {
    let admin = admin::service::check_rpc(&args.password, NAME, &pool).await?;
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
        admin.name,
        args.name,
        serde_json::to_string(&args.value)?,
        element.name(),
        element.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(element)
}
