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
    let cloned_args_user_name = params.user_name.clone();
    let cloned_args_tag_name = params.tag_name.clone();
    let user = pool
        .get()
        .await?
        .interact(move |conn| db::osm_user::queries::select_by_name(&cloned_args_user_name, conn))
        .await??;
    let user = pool
        .get()
        .await?
        .interact(move |conn| {
            db::osm_user::queries::remove_tag(user.id, &cloned_args_tag_name, conn)
        })
        .await??;
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
