use super::model::RpcArea;
use crate::conf::Conf;
use crate::db::user::schema::User;
use crate::Result;
use crate::{area, discord};
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
pub struct Params {
    pub tags: Map<String, Value>,
}

pub async fn run(
    params: Params,
    requesting_user: &User,
    pool: &Pool,
    conf: &Conf,
) -> Result<RpcArea> {
    let area = area::service::insert(params.tags, pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "{} created area {} ({})",
            requesting_user.name,
            area.name(),
            area.id,
        ),
    )
    .await;
    Ok(area.into())
}
