use super::model::RpcArea;
use crate::db::conf::schema::Conf;
use crate::db::user::schema::User;
use crate::service::discord;
use crate::{service, Result};
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
    let area = service::area::insert(params.tags, pool).await?;
    discord::send(
        format!(
            "{} created area {} ({})",
            requesting_user.name,
            area.name(),
            area.id,
        ),
        discord::Channel::Api,
        conf,
    );
    Ok(area.into())
}
