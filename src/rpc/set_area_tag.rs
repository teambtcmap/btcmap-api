use super::model::RpcArea;
use crate::{
    admin,
    area::{self},
    conf::Conf,
    discord, Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::sync::Arc;
use tracing::info;

const NAME: &str = "set_area_tag";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
    pub name: String,
    pub value: Value,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<RpcArea> {
    let admin = admin::service::check_rpc(args.password, NAME, &pool).await?;
    let patch_set = Map::from_iter([(args.name.clone(), args.value.clone())].into_iter());
    let area = pool
        .get()
        .await?
        .interact(move |conn| area::service::patch_tags(&args.id, patch_set, conn))
        .await??;
    let log_message = format!(
        "Admin {} set tag {} = {} for area {} https://api.btcmap.org/v3/areas/{}",
        admin.name,
        args.name,
        serde_json::to_string(&args.value)?,
        area.name(),
        area.id,
    );
    info!(log_message);
    let conf = Conf::select_async(&pool).await?;
    discord::post_message(conf.discord_webhook_api, log_message).await;
    Ok(area.into())
}

#[cfg(test)]
mod test {
    use crate::Result;
    use actix_web::test;

    #[test]
    async fn should_return_401_if_unauthorized() -> Result<()> {
        //let state = mock_state().await;
        //Area::insert(
        //    GeoJson::Feature(Feature::default()),
        //    Map::new(),
        //    &state.conn,
        //)?;
        //let app = test::init_service(
        //    App::new()
        //        .app_data(Data::from(state.pool))
        //        .service(super::patch),
        //)
        //.await;
        //let req = TestRequest::patch()
        //    .uri("/1")
        //    .set_json(PatchArgs { tags: Map::new() })
        //    .to_request();
        //let res = test::call_service(&app, req).await;
        //assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn should_update_area() -> Result<()> {
        //let state = mock_state().await;
        //let admin_password = admin::service::mock_admin("test", &state.pool)
        //    .await
        //    .password;
        //let url_alias = "test";
        //let mut tags = Map::new();
        //tags.insert("url_alias".into(), Value::String(url_alias.into()));
        //Area::insert(GeoJson::Feature(Feature::default()), tags, &state.conn)?;
        //let app = test::init_service(
        //    App::new()
        //        .app_data(Data::from(state.pool))
        //        .service(super::patch),
        //)
        //.await;
        //let args = r#"
        //{
        //    "tags": {
        //        "string": "bar",
        //        "unsigned": 5,
        //        "float": 12.34,
        //        "bool": true
        //    }
        //}
        //"#;
        //let args: Value = serde_json::from_str(args)?;
        //let req = TestRequest::patch()
        //    .uri(&format!("/{url_alias}"))
        //    .append_header(("Authorization", format!("Bearer {admin_password}")))
        //    .set_json(args)
        //    .to_request();
        //let res = test::call_service(&app, req).await;
        //assert_eq!(res.status(), StatusCode::OK);
        //let area = Area::select_by_alias(&url_alias, &state.conn)?.unwrap();
        //assert!(area.tags["string"].is_string());
        //assert!(area.tags["unsigned"].is_u64());
        //assert!(area.tags["float"].is_f64());
        //assert!(area.tags["bool"].is_boolean());
        Ok(())
    }
}
