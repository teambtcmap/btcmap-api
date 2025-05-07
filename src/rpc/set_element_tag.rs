use crate::conf::Conf;
use crate::db::admin::queries::Admin;
use crate::discord;
use crate::element::model::Element;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct Params {
    pub element_id: i64,
    pub tag_name: String,
    pub tag_value: Value,
}

pub async fn run(params: Params, admin: &Admin, pool: &Pool, conf: &Conf) -> Result<Element> {
    let element = Element::select_by_id_async(params.element_id, pool).await?;
    let element =
        Element::set_tag_async(element.id, &params.tag_name, &params.tag_value, pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} set tag {} = {} for element {} ({})",
            admin.name,
            params.tag_name,
            serde_json::to_string(&params.tag_value)?,
            element.name(),
            element.id
        ),
    )
    .await;
    Ok(element)
}
