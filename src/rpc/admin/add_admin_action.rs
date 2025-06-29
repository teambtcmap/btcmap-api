use crate::{
    conf::Conf,
    db::{self, user::schema::User},
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

pub async fn run(params: Params, source_user: &User, pool: &Pool, conf: &Conf) -> Result<Res> {
    let target_user = db::user::queries_async::select_by_name(&params.admin, pool).await?;
    let mut roles = target_user.roles;
    if !roles.contains(&params.action) {
        roles.push(params.action.clone());
    }
    db::user::queries_async::set_roles(target_user.id, &roles, pool).await?;
    discord::send(
        format!(
            "{} added role {} for user {}",
            source_user.name, params.action, target_user.name
        ),
        discord::Channel::Api,
        conf,
    );
    Ok(Res {
        name: target_user.name,
        allowed_actions: roles,
    })
}
