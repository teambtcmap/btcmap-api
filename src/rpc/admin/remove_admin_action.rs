use crate::{conf::Conf, db::user::schema::User, discord, Result};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub admin: String,
    pub action: String,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub allowed_actions: Vec<String>,
}

pub async fn run(params: Params, requesting_user: &User, pool: &Pool, conf: &Conf) -> Result<Res> {
    let target_user = crate::db::user::queries_async::select_by_name(&params.admin, pool).await?;
    let roles: Vec<String> = target_user
        .roles
        .into_iter()
        .filter(|it| it != &params.action)
        .collect();
    crate::db::user::queries_async::set_roles(target_user.id, &roles, pool).await?;
    let target_user = crate::db::user::queries_async::select_by_id(target_user.id, pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "{} removed role {} for user {}",
            requesting_user.name, params.action, target_user.name
        ),
    )
    .await;
    Ok(Res {
        name: target_user.name,
        allowed_actions: target_user.roles,
    })
}
