use super::Admin;
use crate::{conf::Conf, discord, error, Result};
use deadpool_sqlite::Pool;

pub async fn check_rpc(
    password: impl Into<String>,
    action: impl Into<String>,
    pool: &Pool,
) -> Result<Admin> {
    let action = action.into();
    let admin = Admin::select_by_password_async(password, pool).await?;
    if is_allowed(&action, &admin.allowed_actions) {
        Ok(admin)
    } else {
        let conf = Conf::select_async(&pool).await?;
        discord::post_message(
            conf.discord_webhook_api,
            format!(
                "Admin {} tried to call {action} without proper permissions",
                admin.name,
            ),
        )
        .await;
        Err(error::action_is_not_allowed(action))
    }
}

fn is_allowed(action: &str, allowed_actions: &[String]) -> bool {
    (allowed_actions.len() == 1 && allowed_actions.first() == Some(&"all".into()))
        || allowed_actions.contains(&action.into())
}

#[cfg(test)]
mod test {
    use crate::{admin::Admin, test::mock_db, Result};

    #[actix_web::test]
    async fn check_rpc() -> Result<()> {
        let db = mock_db().await;
        assert!(super::check_rpc("pwd", "action", &db.pool).await.is_err());
        let password = "pwd";
        let action = "action";
        Admin::insert("name", password, &db.conn)?;
        Admin::update_allowed_actions(1, &["action".into()], &db.conn)?;
        assert!(super::check_rpc(password, action, &db.pool).await.is_ok());
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
