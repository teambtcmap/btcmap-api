use crate::conf::Conf;
use crate::db::admin::queries::Admin;
use crate::discord;
use crate::element::model::Element;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub element_id: i64,
    pub tag_name: String,
}

pub async fn run(params: Params, admin: &Admin, pool: &Pool, conf: &Conf) -> Result<Element> {
    let element = Element::select_by_id_async(params.element_id, pool).await?;
    let element = Element::remove_tag_async(element.id, &params.tag_name, pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} removed tag {} from element {} ({})",
            admin.name,
            params.tag_name,
            element.name(),
            element.id,
        ),
    )
    .await;
    Ok(element)
}
