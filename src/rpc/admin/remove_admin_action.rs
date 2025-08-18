use std::str::FromStr;

use crate::{
    db::{
        conf::schema::Conf,
        user::schema::{Role, User},
    },
    service::discord,
    Result,
};
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
    let role_to_remove = Role::from_str(&params.action)?;
    let target_user = crate::db::user::queries_async::select_by_name(&params.admin, pool).await?;
    let new_roles: Vec<Role> = target_user
        .roles
        .into_iter()
        .filter(|it| it != &role_to_remove)
        .collect();
    crate::db::user::queries_async::set_roles(target_user.id, &new_roles, pool).await?;
    let target_user = crate::db::user::queries_async::select_by_id(target_user.id, pool).await?;
    discord::send(
        format!(
            "{} removed role {} for user {}",
            requesting_user.name, params.action, target_user.name
        ),
        discord::Channel::Api,
        conf,
    );
    Ok(Res {
        name: target_user.name,
        allowed_actions: target_user
            .roles
            .into_iter()
            .map(|it| it.to_string())
            .collect(),
    })
}
