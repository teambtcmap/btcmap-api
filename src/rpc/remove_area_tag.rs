use super::model::RpcArea;
use crate::{
    conf::Conf,
    db::user::schema::User,
    service::{self, discord},
    Result,
};
use deadpool_sqlite::Pool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub id: String,
    pub tag: String,
}

pub async fn run(params: Params, user: &User, pool: &Pool, conf: &Conf) -> Result<RpcArea> {
    let area = service::area::remove_tag_async(params.id, &params.tag, pool).await?;
    discord::send(
        format!(
            "{} removed tag {} from area {} ({})",
            user.name,
            params.tag,
            area.name(),
            area.id,
        ),
        discord::Channel::Api,
        conf,
    );
    Ok(area.into())
}
