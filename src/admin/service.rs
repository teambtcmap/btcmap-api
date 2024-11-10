use super::Admin;
use crate::Result;
use crate::{discord, Error};
use deadpool_sqlite::Pool;

pub async fn check_rpc(password: &str, action: &str, pool: &Pool) -> Result<Admin> {
    let admin = Admin::select_by_password_async(password, pool)
        .await?
        .ok_or("invalid token")?;
    if !admin.allowed_actions.contains(&"all".into())
        && !admin.allowed_actions.contains(&action.into())
    {
        let log_message = format!(
            "{} tried to call action {} without proper permissions",
            admin.name, action,
        );
        discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
        Err(Error::Unauthorized(
            "You are not allowed to perform this action".into(),
        ))?
    }
    Ok(admin)
}

#[cfg(test)]
mod test {
    use crate::{admin::Admin, test::mock_state, Result};

    #[actix_web::test]
    async fn check_rpc() -> Result<()> {
        let state = mock_state().await;
        assert!(super::check_rpc("pwd", "action", &state.pool)
            .await
            .is_err());
        let password = "pwd";
        let action = "action";
        Admin::insert("name", password, &state.conn)?;
        Admin::update_allowed_actions(1, &vec!["action".into()], &state.conn)?;
        assert!(super::check_rpc(password, action, &state.pool)
            .await
            .is_ok());
        Ok(())
    }
}
