use crate::{admin, conf::Conf, discord, user::User, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::sync::Arc;
use tracing::info;

const NAME: &str = "set_user_tag";

#[derive(Deserialize, Clone)]
pub struct Args {
    pub password: String,
    pub user_name: String,
    pub tag_name: String,
    pub tag_value: Value,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub tags: Map<String, Value>,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let admin = admin::service::check_rpc(args.password, NAME, &pool).await?;
    let cloned_args_user_name = args.user_name.clone();
    let cloned_args_tag_name = args.tag_name.clone();
    let cloned_args_tag_value = args.tag_value.clone();
    let user = pool
        .get()
        .await?
        .interact(move |conn| User::select_by_name(&cloned_args_user_name, conn))
        .await??
        .ok_or(format!("There is no user with name = {}", args.user_name))?;
    let user = pool
        .get()
        .await?
        .interact(move |conn| {
            User::set_tag(user.id, &cloned_args_tag_name, &cloned_args_tag_value, conn)
        })
        .await??;
    let log_message = format!(
        "Admin {} set tag {} = {} for user {} https://api.btcmap.org/v3/users/{}",
        admin.name,
        args.tag_name,
        serde_json::to_string(&args.tag_value)?,
        args.user_name,
        user.id,
    );
    info!(log_message);
    let conf = Conf::select_async(&pool).await?;
    discord::post_message(conf.discord_webhook_api, log_message).await;
    Ok(Res {
        id: user.id,
        tags: user.tags,
    })
}

#[cfg(test)]
mod test {
    use crate::Result;
    use actix_web::test;

    #[test]
    async fn patch_tags() -> Result<()> {
        //let state = mock_state().await;
        //let user = User::insert(1, &OsmUser::mock(), &state.conn)?;
        //let admin_password = admin::service::mock_admin("test", &state.pool)
        //    .await
        //    .password;
        //let app = test::init_service(
        //    App::new()
        //        .app_data(Data::from(state.pool))
        //        .service(super::patch_tags),
        //)
        //.await;
        //let req = TestRequest::patch()
        //    .uri(&format!("/{}/tags", user.id))
        //    .append_header(("Authorization", format!("Bearer {admin_password}")))
        //    .set_json(json!({ "foo": "bar" }))
        //    .to_request();
        //let res = test::call_service(&app, req).await;
        //assert_eq!(res.status(), StatusCode::OK);
        Ok(())
    }
}
