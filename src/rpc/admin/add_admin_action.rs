use crate::{
    db::{self, user::schema::Role},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let new_role = Role::from_str(&params.action)?;
    let target_user = db::user::queries::select_by_name(&params.admin, pool).await?;
    let mut roles = target_user.roles;
    if !roles.contains(&new_role) {
        roles.push(new_role);
    }
    db::user::queries::set_roles(target_user.id, &roles, pool).await?;
    Ok(Res {
        name: target_user.name,
        allowed_actions: roles.into_iter().map(|it| it.to_string()).collect(),
    })
}
