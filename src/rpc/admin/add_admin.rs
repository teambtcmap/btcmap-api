use crate::{
    db::{conf::schema::Conf, user::schema::User},
    service::discord,
    Result,
};
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
    let new_user = crate::db::user::queries_async::insert(
        params.new_admin_name,
        params.new_admin_password,
        pool,
    )
    .await?;
    discord::send(
        format!("{} added new user {}", requesting_user.name, new_user.name),
        discord::Channel::Api,
        conf,
    );
    Ok(Res {
        name: new_user.name,
        allowed_actions: new_user.roles,
    })
}
