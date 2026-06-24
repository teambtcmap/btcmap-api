use crate::db;
use crate::service::auth::{MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH};
use crate::Result;
use argon2::PasswordHasher;
use argon2::PasswordVerifier;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub username: String,
    pub old_password: String,
    pub new_password: String,
}

#[derive(Serialize)]
pub struct Res {
    pub changed: bool,
}

impl std::fmt::Debug for Res {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Res")
            .field("changed", &self.changed)
            .finish()
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    if params.new_password.len() < MIN_PASSWORD_LENGTH {
        return Err(
            format!("New password is too short, use at least {MIN_PASSWORD_LENGTH} chars").into(),
        );
    }
    if params.new_password.len() > MAX_PASSWORD_LENGTH {
        return Err(
            format!("New password is too long, use at most {MAX_PASSWORD_LENGTH} chars").into(),
        );
    }
    let err_invalid_username_password = "Incorrect username or password";
    let err_generic = "Unexpected error, please contact administrator";
    let user = db::main::user::queries::select_by_name(params.username, pool)
        .await
        .map_err(|_| err_invalid_username_password)?;
    let old_password_hash = PasswordHash::new(&user.password).map_err(|_| err_generic)?;
    Argon2::default()
        .verify_password(params.old_password.as_bytes(), &old_password_hash)
        .map_err(|_| err_invalid_username_password)?;
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(params.new_password.as_bytes(), &salt)
        .map_err(|_| err_generic)?
        .to_string();
    db::main::user::queries::set_password(user.id, password_hash, pool)
        .await
        .map_err(|_| err_generic)?;
    Ok(Res { changed: true })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::main::test::pool;
    use crate::rpc::auth::signin;

    fn hash_password(password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string()
    }

    async fn insert_user(
        name: &str,
        password: &str,
        pool: &Pool,
    ) -> crate::db::main::user::schema::User {
        db::main::user::queries::insert(name, hash_password(password), pool)
            .await
            .unwrap()
    }

    async fn soft_delete_user(id: i64, pool: &Pool) {
        pool.get()
            .await
            .unwrap()
            .interact(move |conn| {
                conn.execute(
                    "UPDATE user SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ') WHERE id = ?1",
                    rusqlite::params![id],
                )
                .unwrap();
            })
            .await
            .unwrap();
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

    fn long_enough(s: &str) -> String {
        assert!(s.len() >= MIN_PASSWORD_LENGTH);
        s.to_string()
    }

    #[test]
    fn successful_password_change_updates_hash() -> Result<()> {
        run_test(async {
            let pool = pool();
            insert_user("satoshi", "ihsotasatoshi123", &pool).await;

            let res = super::run(
                Params {
                    username: "satoshi".into(),
                    old_password: "ihsotasatoshi123".into(),
                    new_password: long_enough("newpassfoobarbaz"),
                },
                &pool,
            )
            .await?;
            assert!(res.changed);

            let stored = db::main::user::queries::select_by_name("satoshi", &pool).await?;
            assert_ne!(stored.password, "ihsotasatoshi123");
            assert!(PasswordHash::new(&stored.password).is_ok());
            Ok(())
        })
    }

    #[test]
    fn after_change_old_password_no_longer_signs_in() -> Result<()> {
        run_test(async {
            let pool = pool();
            insert_user("satoshi", "ihsotasatoshi123", &pool).await;

            super::run(
                Params {
                    username: "satoshi".into(),
                    old_password: "ihsotasatoshi123".into(),
                    new_password: long_enough("newpassfoobarbaz"),
                },
                &pool,
            )
            .await?;

            let err = signin::run(
                signin::Params {
                    username: "satoshi".into(),
                    password: "ihsotasatoshi123".into(),
                    label: None,
                    roles: vec![],
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(err.to_string(), "Invalid credentials");

            signin::run(
                signin::Params {
                    username: "satoshi".into(),
                    password: "newpassfoobarbaz".into(),
                    label: None,
                    roles: vec![],
                },
                &pool,
            )
            .await?;
            Ok(())
        })
    }

    #[test]
    fn wrong_old_password_returns_masked_error() -> Result<()> {
        run_test(async {
            let pool = pool();
            insert_user("satoshi", "ihsotasatoshi123", &pool).await;

            let err = super::run(
                Params {
                    username: "satoshi".into(),
                    old_password: "wrong-old-password".into(),
                    new_password: long_enough("newpassfoobarbaz"),
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(err.to_string(), "Incorrect username or password");
            Ok(())
        })
    }

    #[test]
    fn unknown_user_returns_same_masked_error() -> Result<()> {
        run_test(async {
            let pool = pool();

            let err = super::run(
                Params {
                    username: "ghost".into(),
                    old_password: "ihsotasatoshi123".into(),
                    new_password: long_enough("newpassfoobarbaz"),
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(err.to_string(), "Incorrect username or password");
            Ok(())
        })
    }

    #[test]
    fn soft_deleted_user_cannot_change_password() -> Result<()> {
        run_test(async {
            let pool = pool();
            let user = insert_user("satoshi", "ihsotasatoshi123", &pool).await;
            soft_delete_user(user.id, &pool).await;

            let err = super::run(
                Params {
                    username: "satoshi".into(),
                    old_password: "ihsotasatoshi123".into(),
                    new_password: long_enough("newpassfoobarbaz"),
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(err.to_string(), "Incorrect username or password");
            Ok(())
        })
    }

    #[test]
    fn new_password_too_short_returns_validation_error() -> Result<()> {
        run_test(async {
            let pool = pool();
            insert_user("satoshi", "ihsotasatoshi123", &pool).await;

            let err = super::run(
                Params {
                    username: "satoshi".into(),
                    old_password: "ihsotasatoshi123".into(),
                    new_password: "short".into(),
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(
                err.to_string(),
                format!("New password is too short, use at least {MIN_PASSWORD_LENGTH} chars")
            );
            Ok(())
        })
    }

    #[test]
    fn new_password_too_long_returns_validation_error() -> Result<()> {
        run_test(async {
            let pool = pool();
            insert_user("satoshi", "ihsotasatoshi123", &pool).await;

            let too_long = "x".repeat(MAX_PASSWORD_LENGTH + 1);
            let err = super::run(
                Params {
                    username: "satoshi".into(),
                    old_password: "ihsotasatoshi123".into(),
                    new_password: too_long,
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(
                err.to_string(),
                format!("New password is too long, use at most {MAX_PASSWORD_LENGTH} chars")
            );
            Ok(())
        })
    }
}
