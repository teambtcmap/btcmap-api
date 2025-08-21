use crate::{
    db::{self, conf::schema::Conf, user::schema::User},
    service::discord,
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub element_id: i64,
    pub comment: String,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
}

pub async fn run(params: Params, requesting_user: &User, pool: &Pool, conf: &Conf) -> Result<Res> {
    let element = db::element::queries::select_by_id(params.element_id, pool).await?;
    let comment =
        db::element_comment::queries_async::insert(element.id, &params.comment, pool).await?;
    discord::send(
        format!(
            "{} added a comment to element {} ({}): {}",
            requesting_user.name,
            element.name(),
            element.id,
            params.comment,
        ),
        discord::Channel::Api,
        conf,
    );
    Ok(Res { id: comment.id })
}
