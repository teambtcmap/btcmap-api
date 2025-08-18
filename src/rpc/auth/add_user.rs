use crate::{
    db::{self, conf::schema::Conf, user::schema::Role},
    service::discord,
    Result,
};
use argon2::PasswordHasher;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub name: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub roles: Vec<String>,
}

pub async fn run(params: Params, pool: &Pool, conf: &Conf) -> Result<Res> {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(params.password.as_bytes(), &salt)
        .map_err(|e| e.to_string())?
        .to_string();
    let user = crate::db::user::queries_async::insert(params.name, password_hash, pool).await?;
    let user = db::user::queries_async::set_roles(user.id, &[Role::User], pool).await?;
    discord::send(
        format!("New user: {}", user.name),
        discord::Channel::Api,
        conf,
    );
    Ok(Res {
        name: user.name,
        roles: user.roles.into_iter().map(|it| it.to_string()).collect(),
    })
}
