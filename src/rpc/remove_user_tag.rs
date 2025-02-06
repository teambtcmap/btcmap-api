use crate::{admin::Admin, conf::Conf, discord, user::User, Result};
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

pub async fn run(params: Params, admin: &Admin, pool: &Pool, conf: &Conf) -> Result<Res> {
    let cloned_args_user_name = params.user_name.clone();
    let cloned_args_tag_name = params.tag_name.clone();
    let user = pool
        .get()
        .await?
        .interact(move |conn| User::select_by_name(&cloned_args_user_name, conn))
        .await??
        .ok_or(format!("There is no user with name = {}", params.user_name))?;
    let user = pool
        .get()
        .await?
        .interact(move |conn| User::remove_tag(user.id, &cloned_args_tag_name, conn))
        .await??
        .ok_or(format!("There is no user with name = {}", params.user_name))?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} removed tag {} for user {} ({})",
            admin.name, params.tag_name, params.user_name, user.id,
        ),
    )
    .await;
    Ok(Res {
        id: user.id,
        tags: user.tags,
    })
}
