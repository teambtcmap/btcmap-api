use crate::admin::Admin;
use crate::conf::Conf;
use crate::discord;
use crate::element::model::Element;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub id: String,
    pub tag: String,
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
    let element = Element::remove_tag_async(element.id, &params.tag, pool).await?;
    let log_message = format!(
        "Admin {} removed tag {} from element {} https://api.btcmap.org/v4/elements/{}",
        admin.name,
        params.tag,
        element.name(),
        element.id,
    );
    discord::post_message(&conf.discord_webhook_api, log_message).await;
    Ok(element)
}
