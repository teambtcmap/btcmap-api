use crate::{
    db::{self, main::user::schema::Role},
    service::auth::{MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH},
    Result,
};
use argon2::PasswordHasher;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2,
};
use deadpool_sqlite::Pool;
use names::Generator;
use names::Name;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Params {
    pub username: Option<String>,
    pub password: String,
    pub label: Option<String>,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub roles: Vec<String>,
    pub api_key: String,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    if params.password.len() < MIN_PASSWORD_LENGTH {
        return Err(
            format!("Password is too short, use at least {MIN_PASSWORD_LENGTH} chars").into(),
        );
    }
    if params.password.len() > MAX_PASSWORD_LENGTH {
        return Err(
            format!("Password is too long, use at most {MAX_PASSWORD_LENGTH} chars").into(),
        );
    }
    let name = match params.username {
        Some(n) => n,
        None => Generator::with_naming(Name::Numbered)
            .next()
            .unwrap_or_default(),
    };
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(params.password.as_bytes(), &salt)
        .map_err(|e| e.to_string())?
        .to_string();
    let user = crate::db::main::user::queries::insert(&name, password_hash, pool).await?;
    let user = db::main::user::queries::set_roles(user.id, &[Role::User], pool).await?;
    let token = Uuid::new_v4().to_string();
    db::main::access_token::queries::insert(
        user.id,
        params.label.unwrap_or_default(),
        token.clone(),
        vec![],
        pool,
    )
    .await?;
    Ok(Res {
        name: user.name,
        roles: user.roles.into_iter().map(|it| it.to_string()).collect(),
        api_key: token,
    })
}
