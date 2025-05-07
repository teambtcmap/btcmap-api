use crate::{
    conf::Conf, db::admin::queries::Admin, discord, element::Element,
    element_comment::ElementComment, Result,
};
use deadpool_sqlite::Pool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub element_id: i64,
    pub comment: String,
}

pub async fn run(
    params: Params,
    admin: &Admin,
    pool: &Pool,
    conf: &Conf,
) -> Result<ElementComment> {
    let element = Element::select_by_id_async(params.element_id, pool).await?;
    let comment = ElementComment::insert_async(element.id, &params.comment, pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} added a comment to element {} ({}): {}",
            admin.name,
            element.name(),
            element.id,
            params.comment,
        ),
    )
    .await;
    Ok(comment)
}
