use crate::db;
use crate::db::main::user::schema::Role;
use crate::rpc::handler::allowed_methods;
use crate::{Error, Result};
use argon2::PasswordVerifier;
use argon2::{Argon2, PasswordHash};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Deserialize, Clone)]
pub struct Params {
    pub username: String,
    pub password: String,
    pub label: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Serialize)]
pub struct Res {
    pub api_key: String,
    pub roles: Vec<String>,
}

impl std::fmt::Debug for Res {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Res")
            .field("api_key", &"<redacted>")
            .field("roles", &self.roles)
            .finish()
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let error_cause_mask = "Invalid credentials";
    let user = db::main::user::queries::select_by_name(params.username, pool)
        .await
        .map_err(|_| error_cause_mask)?;
    let password_hash = PasswordHash::new(&user.password).map_err(|_| error_cause_mask)?;
    Argon2::default()
        .verify_password(params.password.as_bytes(), &password_hash)
        .map_err(|_| error_cause_mask)?;
    let token_roles = parse_and_validate_token_roles(&params.roles, &user.roles)?;
    let api_key = Uuid::new_v4().to_string();
    db::main::access_token::queries::insert(
        user.id,
        params.label.unwrap_or_default(),
        api_key.clone(),
        token_roles,
        pool,
    )
    .await?;
    Ok(Res {
        api_key,
        roles: params.roles,
    })
}

fn parse_and_validate_token_roles(requested: &[String], user_roles: &[Role]) -> Result<Vec<Role>> {
    let mut parsed = Vec::with_capacity(requested.len());
    for name in requested {
        let role = Role::from_str(name)
            .map_err(|_| Error::Other(format!("'{}' is not a valid Role", name)))?;
        parsed.push(role);
    }
    if !parsed.is_empty() {
        let user_methods = allowed_methods(user_roles);
        let token_methods = allowed_methods(&parsed);
        if !token_methods.is_subset(&user_methods) {
            return Err(Error::Other(
                "Requested token roles exceed the scope of the user record".into(),
            ));
        }
    }
    Ok(parsed)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::main::test::pool;
    use argon2::PasswordHasher;
    use argon2::{
        password_hash::{rand_core::OsRng, SaltString},
        Argon2,
    };

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

    async fn set_user_roles(id: i64, roles: &[Role], pool: &Pool) {
        db::main::user::queries::set_roles(id, roles, pool)
            .await
            .unwrap();
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

    #[test]
    fn successful_signin_returns_api_key() -> Result<()> {
        run_test(async {
            let pool = pool();
            insert_user("satoshi", "ihsotasatoshi123", &pool).await;

            let res = super::run(
                Params {
                    username: "satoshi".into(),
                    password: "ihsotasatoshi123".into(),
                    label: None,
                    roles: vec![],
                },
                &pool,
            )
            .await?;
            assert!(!res.api_key.is_empty());
            assert!(res.roles.is_empty());
            Ok(())
        })
    }

    #[test]
    fn wrong_password_returns_masked_error() -> Result<()> {
        run_test(async {
            let pool = pool();
            insert_user("satoshi", "ihsotasatoshi123", &pool).await;

            let err = super::run(
                Params {
                    username: "satoshi".into(),
                    password: "wrong-password".into(),
                    label: None,
                    roles: vec![],
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(err.to_string(), "Invalid credentials");
            Ok(())
        })
    }

    #[test]
    fn unknown_user_returns_same_masked_error_as_wrong_password() -> Result<()> {
        run_test(async {
            let pool = pool();

            let err = super::run(
                Params {
                    username: "ghost".into(),
                    password: "ihsotasatoshi123".into(),
                    label: None,
                    roles: vec![],
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(err.to_string(), "Invalid credentials");
            Ok(())
        })
    }

    #[test]
    fn soft_deleted_user_cannot_signin() -> Result<()> {
        run_test(async {
            let pool = pool();
            let user = insert_user("satoshi", "ihsotasatoshi123", &pool).await;
            soft_delete_user(user.id, &pool).await;

            let err = super::run(
                Params {
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
            Ok(())
        })
    }

    #[test]
    fn label_is_persisted_on_issued_token() -> Result<()> {
        run_test(async {
            let pool = pool();
            insert_user("satoshi", "ihsotasatoshi123", &pool).await;

            super::run(
                Params {
                    username: "satoshi".into(),
                    password: "ihsotasatoshi123".into(),
                    label: Some("my laptop".into()),
                    roles: vec![],
                },
                &pool,
            )
            .await?;

            let tokens = db::main::access_token::queries::select_by_user_id(1, &pool).await?;
            assert_eq!(tokens.len(), 1);
            assert_eq!(tokens[0].label.as_deref(), Some("my laptop"));
            Ok(())
        })
    }

    #[test]
    fn admin_user_can_mint_dashboard_token() -> Result<()> {
        run_test(async {
            let pool = pool();
            let user = insert_user("admin1", "ihsotasatoshi123", &pool).await;
            set_user_roles(user.id, &[Role::Admin], &pool).await;

            let res = super::run(
                Params {
                    username: "admin1".into(),
                    password: "ihsotasatoshi123".into(),
                    label: Some("dashboard".into()),
                    roles: vec!["dashboard".into()],
                },
                &pool,
            )
            .await?;
            assert!(!res.api_key.is_empty());
            assert_eq!(res.roles, vec!["dashboard".to_string()]);

            let token =
                db::main::access_token::queries::select_by_secret(res.api_key, &pool).await?;
            assert_eq!(token.roles, vec![Role::Dashboard]);
            Ok(())
        })
    }

    #[test]
    fn root_user_can_mint_dashboard_token() -> Result<()> {
        run_test(async {
            let pool = pool();
            let user = insert_user("root1", "ihsotasatoshi123", &pool).await;
            set_user_roles(user.id, &[Role::Root], &pool).await;

            let res = super::run(
                Params {
                    username: "root1".into(),
                    password: "ihsotasatoshi123".into(),
                    label: Some("dashboard".into()),
                    roles: vec!["dashboard".into()],
                },
                &pool,
            )
            .await?;
            let token =
                db::main::access_token::queries::select_by_secret(res.api_key, &pool).await?;
            assert_eq!(token.roles, vec![Role::Dashboard]);
            Ok(())
        })
    }

    #[test]
    fn regular_user_cannot_mint_dashboard_token() -> Result<()> {
        run_test(async {
            let pool = pool();
            insert_user("alice", "ihsotasatoshi123", &pool).await;

            let err = super::run(
                Params {
                    username: "alice".into(),
                    password: "ihsotasatoshi123".into(),
                    label: Some("dashboard".into()),
                    roles: vec!["dashboard".into()],
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(
                err.to_string(),
                "Requested token roles exceed the scope of the user record"
            );
            Ok(())
        })
    }

    #[test]
    fn regular_user_cannot_mint_admin_token() -> Result<()> {
        run_test(async {
            let pool = pool();
            insert_user("alice", "ihsotasatoshi123", &pool).await;

            let err = super::run(
                Params {
                    username: "alice".into(),
                    password: "ihsotasatoshi123".into(),
                    label: Some("admin".into()),
                    roles: vec!["admin".into()],
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(
                err.to_string(),
                "Requested token roles exceed the scope of the user record"
            );
            Ok(())
        })
    }

    #[test]
    fn admin_user_cannot_mint_root_token() -> Result<()> {
        run_test(async {
            let pool = pool();
            let user = insert_user("admin1", "ihsotasatoshi123", &pool).await;
            set_user_roles(user.id, &[Role::Admin], &pool).await;

            let err = super::run(
                Params {
                    username: "admin1".into(),
                    password: "ihsotasatoshi123".into(),
                    label: Some("root".into()),
                    roles: vec!["root".into()],
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(
                err.to_string(),
                "Requested token roles exceed the scope of the user record"
            );
            Ok(())
        })
    }

    #[test]
    fn invalid_role_string_is_rejected() -> Result<()> {
        run_test(async {
            let pool = pool();
            insert_user("alice", "ihsotasatoshi123", &pool).await;

            let err = super::run(
                Params {
                    username: "alice".into(),
                    password: "ihsotasatoshi123".into(),
                    label: None,
                    roles: vec!["superuser".into()],
                },
                &pool,
            )
            .await
            .unwrap_err();
            assert_eq!(err.to_string(), "'superuser' is not a valid Role");
            Ok(())
        })
    }

    #[test]
    fn omitted_roles_field_preserves_legacy_behavior() -> Result<()> {
        run_test(async {
            let pool = pool();
            insert_user("alice", "ihsotasatoshi123", &pool).await;

            let raw = r#"{"username":"alice","password":"ihsotasatoshi123"}"#;
            let params: Params = serde_json::from_str(raw).unwrap();
            let res = super::run(params, &pool).await?;
            assert!(res.roles.is_empty());

            let token =
                db::main::access_token::queries::select_by_secret(res.api_key, &pool).await?;
            assert!(token.roles.is_empty());
            Ok(())
        })
    }
}
