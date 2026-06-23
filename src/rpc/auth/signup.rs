use crate::{
    db::{self, main::user::schema::Role},
    service::auth::{MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH},
    Error, Result,
};
use argon2::PasswordHasher;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2,
};
use deadpool_sqlite::Pool;
use names::Generator;
use names::Name;
use rusqlite::ffi::ErrorCode;
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

impl std::fmt::Debug for Res {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Res")
            .field("name", &self.name)
            .field("roles", &self.roles)
            .field("api_key", &"<redacted>")
            .finish()
    }
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
            .ok_or_else(|| {
                Error::Other("Failed to generate a username, please provide one explicitly".into())
            })?,
    };
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(params.password.as_bytes(), &salt)
        .map_err(|e| e.to_string())?
        .to_string();
    let user = match crate::db::main::user::queries::insert(&name, password_hash, pool).await {
        Ok(user) => user,
        Err(Error::Rusqlite(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::ConstraintViolation,
                ..
            },
            _,
        ))) => {
            return Err(Error::Other("Username already taken".into()));
        }
        Err(_) => {
            return Err(Error::Other(
                "Unexpected error, please contact administrator".into(),
            ));
        }
    };
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::main::test::pool;

    fn long_enough(s: &str) -> String {
        assert!(s.len() >= MIN_PASSWORD_LENGTH);
        s.to_string()
    }

    fn run_test<F>(future: F) -> Result<()>
    where
        F: std::future::Future<Output = Result<()>>,
    {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn signup_creates_user_with_user_role_and_api_key() -> Result<()> {
        run_test(async {
            let pool = pool();
            let res = super::run(
                Params {
                    username: Some("satoshi".into()),
                    password: long_enough("ihsotasatoshi123"),
                    label: Some("cli".into()),
                },
                &pool,
            )
            .await?;
            assert_eq!(res.name, "satoshi");
            assert_eq!(res.roles, vec!["user".to_string()]);
            assert!(!res.api_key.is_empty());

            let token =
                db::main::access_token::queries::select_by_secret(res.api_key, &pool).await?;
            assert_eq!(token.name.as_deref(), Some("cli"));
            Ok(())
        })
    }

    #[test]
    fn signup_with_omitted_username_generates_one() -> Result<()> {
        run_test(async {
            let pool = pool();
            let res = super::run(
                Params {
                    username: None,
                    password: long_enough("ihsotasatoshi123"),
                    label: None,
                },
                &pool,
            )
            .await?;
            assert!(!res.name.is_empty());
            assert_eq!(res.roles, vec!["user".to_string()]);
            Ok(())
        })
    }

    #[test]
    fn duplicate_username_returns_friendly_error() -> Result<()> {
        run_test(async {
            let pool = pool();
            super::run(
                Params {
                    username: Some("satoshi".into()),
                    password: long_enough("ihsotasatoshi123"),
                    label: None,
                },
                &pool,
            )
            .await?;

            let err = super::run(
                Params {
                    username: Some("satoshi".into()),
                    password: long_enough("anothersatoshi123"),
                    label: None,
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(err.to_string(), "Username already taken");
            Ok(())
        })
    }

    #[test]
    fn password_too_short_returns_validation_error() -> Result<()> {
        run_test(async {
            let pool = pool();
            let err = super::run(
                Params {
                    username: Some("satoshi".into()),
                    password: "short".into(),
                    label: None,
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(
                err.to_string(),
                format!("Password is too short, use at least {MIN_PASSWORD_LENGTH} chars")
            );
            Ok(())
        })
    }

    #[test]
    fn password_too_long_returns_validation_error() -> Result<()> {
        run_test(async {
            let pool = pool();
            let too_long = "x".repeat(MAX_PASSWORD_LENGTH + 1);
            let err = super::run(
                Params {
                    username: Some("satoshi".into()),
                    password: too_long,
                    label: None,
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(
                err.to_string(),
                format!("Password is too long, use at most {MAX_PASSWORD_LENGTH} chars")
            );
            Ok(())
        })
    }
}
