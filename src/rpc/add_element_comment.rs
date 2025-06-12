use crate::{
    conf::Conf,
    db::{self, user::schema::User},
    discord,
    element_comment::ElementComment,
    Result,
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
    requesting_user: &User,
    pool: &Pool,
    conf: &Conf,
) -> Result<ElementComment> {
    let element = db::element::queries_async::select_by_id(params.element_id, pool).await?;
    let comment = ElementComment::insert_async(element.id, &params.comment, pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "{} added a comment to element {} ({}): {}",
            requesting_user.name,
            element.name(),
            element.id,
            params.comment,
        ),
    )
    .await;
    Ok(comment)
}
