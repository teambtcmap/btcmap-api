use crate::conf::Conf;
use crate::db::user::schema::User;
use crate::element::model::Element;
use crate::Result;
use crate::{db, discord};
use deadpool_sqlite::Pool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub element_id: i64,
    pub tag_name: String,
}

pub async fn run(params: Params, user: &User, pool: &Pool, conf: &Conf) -> Result<Element> {
    let element = db::element::queries_async::select_by_id(params.element_id, pool).await?;
    let element = Element::remove_tag_async(element.id, &params.tag_name, pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "{} removed tag {} from element {} ({})",
            user.name,
            params.tag_name,
            element.name(),
            element.id,
        ),
    )
    .await;
    Ok(element)
}
