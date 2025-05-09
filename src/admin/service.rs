use crate::db::admin;
use crate::{conf::Conf, db::admin::queries::Admin, discord, error::Error, Result};
use deadpool_sqlite::Pool;

pub async fn check_rpc(
    password: impl Into<String>,
    action: impl Into<String>,
    pool: &Pool,
) -> Result<Admin> {
    let action = action.into();
    let admin = admin::queries_async::select_by_password(password, pool).await?;
    if is_allowed(&action, &admin.roles) {
        Ok(admin)
    } else {
        let conf = Conf::select_async(pool).await?;
        discord::post_message(
            conf.discord_webhook_api,
            format!(
                "Admin {} tried to call {action} without proper permissions",
                admin.name,
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
    use crate::db::admin;
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
