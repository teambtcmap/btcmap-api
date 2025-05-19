use crate::db::{access_token, admin};
use crate::{conf::Conf, db::admin::queries::Admin, discord, error::Error};
use crate::{db, Result};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::PasswordHasher;
use argon2::{Argon2, PasswordHash};
use deadpool_sqlite::Pool;
use tracing::warn;

pub async fn upgrade_plaintext_passwords(pool: &Pool) -> Result<()> {
    let admins = db::admin::queries_async::select_all(pool).await?;
    warn!("Loaded {} admin users", admins.len());
    for admin in &admins {
        let parsed_hash = PasswordHash::new(&admin.password);
        if parsed_hash.is_err() {
            warn!("User {} has plaintext password", admin.name);
            let argon2 = Argon2::default();
            let salt = SaltString::generate(&mut OsRng);
            let password_hash = argon2
                .hash_password(admin.password.as_bytes(), &salt)
                .unwrap()
                .to_string();
            warn!("Generated password hash {password_hash}");
            db::admin::queries_async::set_password(admin.id, password_hash, pool).await?;
            warn!("Saved hash");
        }
    }
    Ok(())
}

pub async fn check_rpc(
    password: impl Into<String>,
    action: impl Into<String>,
    pool: &Pool,
) -> Result<Admin> {
    let action = action.into();
    let token = access_token::queries_async::select_by_secret(password, pool).await?;
    let a = admin::queries_async::select_by_id(token.user_id, pool).await?;
    if is_allowed(&action, &token.roles) {
        Ok(a)
    } else {
        let conf = Conf::select_async(pool).await?;
        discord::post_message(
            conf.discord_webhook_api,
            format!(
                "Admin {} tried to call {action} without proper permissions",
                a.name,
            ),
        )
        .await;
        Err(Error::unauthorized(action))
    }
}

fn is_allowed(action: &str, allowed_actions: &[String]) -> bool {
    (allowed_actions.len() == 1 && allowed_actions.first() == Some(&"all".into()))
        || allowed_actions.contains(&action.into())
}

#[cfg(test)]
mod test {
    use crate::db::{access_token, admin};
    use crate::test::mock_pool;
    use crate::Result;

    #[actix_web::test]
    async fn check_rpc() -> Result<()> {
        let pool = mock_pool().await;
        assert!(super::check_rpc("pwd", "action", &pool).await.is_err());
        let password = "pwd";
        let action = "action";
        admin::queries_async::insert("name", password, &pool).await?;
        admin::queries_async::set_roles(1, &["action".into()], &pool).await?;
        let admin = admin::queries_async::select_by_id(1, &pool).await?;
        access_token::queries_async::insert(admin.id, "", password, &admin.roles, &pool).await?;
        assert!(super::check_rpc(password, action, &pool).await.is_ok());
        Ok(())
    }

    #[test]
    fn is_allowed() -> Result<()> {
        let mut allowed_actions: Vec<String> =
            vec!["action_1".into(), "action_2".into(), "action_3".into()];
        assert!(super::is_allowed("action_2", &allowed_actions));
        assert!(!super::is_allowed("action_4", &allowed_actions));
        allowed_actions.clear();
        allowed_actions.push("all".into());
        assert!(super::is_allowed("action_1", &allowed_actions));
        Ok(())
    }
}
