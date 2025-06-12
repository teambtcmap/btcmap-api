use crate::{conf::Conf, db::user::schema::User, discord, Result};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub new_admin_name: String,
    pub new_admin_password: String,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub allowed_actions: Vec<String>,
}

pub async fn run(params: Params, requesting_user: &User, pool: &Pool, conf: &Conf) -> Result<Res> {
    let new_admin_id = crate::db::user::queries_async::insert(
        params.new_admin_name,
        params.new_admin_password,
        pool,
    )
    .await?;
    let new_user = crate::db::user::queries_async::select_by_id(new_admin_id, pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!("{} added new user {}", requesting_user.name, new_user.name),
    )
    .await;
    Ok(Res {
        name: new_user.name,
        allowed_actions: new_user.roles,
    })
}
