use crate::admin::Admin;
use crate::conf::Conf;
use crate::discord;
use crate::element::model::Element;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Clone)]
pub struct Params {
    pub id: String,
    pub name: String,
    pub value: Value,
}

pub async fn run_internal(
    params: Params,
    admin: &Admin,
    pool: &Pool,
    conf: &Conf,
) -> Result<Element> {
    let element = Element::select_by_id_or_osm_id_async(params.id, pool)
        .await?
        .ok_or("Element not found")?;
    Element::set_tag_async(element.id, &params.name, &params.value, pool).await?;
    let message = format!(
        "Admin {} set tag {} = {} for element {} https://api.btcmap.org/v4/elements/{}",
        admin.name,
        params.name,
        serde_json::to_string(&params.value)?,
        element.name(),
        element.id,
    );
    discord::post_message(&conf.discord_webhook_api, message).await;
    Ok(element)
}
