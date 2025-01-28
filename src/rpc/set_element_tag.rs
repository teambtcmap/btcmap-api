use crate::conf::Conf;
use crate::discord;
use crate::Result;
use crate::{admin, element::model::Element};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

pub const NAME: &str = "set_element_tag";

#[derive(Deserialize, Clone)]
pub struct Args {
    pub password: String,
    pub id: String,
    pub name: String,
    pub value: Value,
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
            args.id
        ))?;
    let cloned_name = args.name.clone();
    let cloned_value = args.value.clone();
    let element = pool
        .get()
        .await?
        .interact(move |conn| Element::set_tag(element.id, &cloned_name, &cloned_value, conn))
        .await??;
    let log_message = format!(
        "Admin {} set tag {} = {} for element {} https://api.btcmap.org/v3/elements/{}",
        admin.name,
        args.name,
        serde_json::to_string(&args.value)?,
        element.name(),
        element.id,
    );
    info!(log_message);
    let conf = Conf::select_async(&pool).await?;
    discord::post_message(conf.discord_webhook_api, log_message).await;
    Ok(element)
}
