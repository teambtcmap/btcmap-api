use crate::{
    db::{self, conf::schema::Conf, user::schema::User},
    service::discord,
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Deserialize, Clone)]
pub struct Params {
    pub user_name: String,
    pub tag_name: String,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub tags: Map<String, Value>,
}

pub async fn run(params: Params, caller: &User, pool: &Pool, conf: &Conf) -> Result<Res> {
    let user = db::osm_user::queries::select_by_name(params.user_name.clone(), pool).await?;
    let user = db::osm_user::queries::remove_tag(user.id, params.tag_name.clone(), pool).await?;
    discord::send(
        format!(
            "{} removed tag {} for user {} ({})",
            caller.name, params.tag_name, params.user_name, user.id,
        ),
        discord::Channel::Api,
        conf,
    );
    Ok(Res {
        id: user.id,
        tags: user.tags,
    })
}
