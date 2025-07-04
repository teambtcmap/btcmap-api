use crate::{
    conf::Conf,
    db::{self, user::schema::User},
    service::discord,
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Deserialize, Clone)]
pub struct Params {
    pub user_name: String,
    pub tag_name: String,
    pub tag_value: Value,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub tags: Map<String, Value>,
}

pub async fn run(params: Params, caller: &User, pool: &Pool, conf: &Conf) -> Result<Res> {
    let cloned_args_user_name = params.user_name.clone();
    let cloned_args_tag_name = params.tag_name.clone();
    let cloned_args_tag_value = params.tag_value.clone();
    let user = pool
        .get()
        .await?
        .interact(move |conn| db::osm_user::queries::select_by_name(&cloned_args_user_name, conn))
        .await??;
    let user = pool
        .get()
        .await?
        .interact(move |conn| {
            db::osm_user::queries::set_tag(
                user.id,
                &cloned_args_tag_name,
                &cloned_args_tag_value,
                conn,
            )
        })
        .await??;
    let discord_message = format!(
        "{} set tag {} = {} for user {} https://api.btcmap.org/v3/users/{}",
        caller.name,
        params.tag_name,
        serde_json::to_string(&params.tag_value)?,
        params.user_name,
        user.id,
    );
    discord::send(discord_message, discord::Channel::Api, conf);
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
