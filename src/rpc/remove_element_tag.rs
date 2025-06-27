use crate::conf::Conf;
use crate::db::user::schema::User;
use crate::Result;
use crate::{db, discord};
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub element_id: i64,
    pub tag_name: String,
}

#[derive(Serialize)]
pub struct Res {
    id: i64,
    tags: JsonObject,
}

pub async fn run(params: Params, user: &User, pool: &Pool, conf: &Conf) -> Result<Res> {
    let element = db::element::queries_async::select_by_id(params.element_id, pool).await?;
    let element =
        db::element::queries_async::remove_tag(element.id, &params.tag_name, pool).await?;
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
    Ok(Res {
        id: element.id,
        tags: element.tags,
    })
}
