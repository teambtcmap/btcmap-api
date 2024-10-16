use super::model::RpcArea;
use crate::Result;
use crate::{admin, area, discord};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

const NAME: &str = "remove_area";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub id: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<RpcArea> {
    let admin = admin::service::check_rpc(&args.password, NAME, &pool).await?;
    let area = pool
        .get()
        .await?
        .interact(move |conn| area::service::soft_delete(&args.id, conn))
        .await??;
    let log_message = format!(
        "{} removed area {} https://api.btcmap.org/v3/areas/{}",
        admin.name,
        area.name(),
        area.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area.into())
}

#[cfg(test)]
mod test {
    use crate::Result;

    #[tokio::test]
    async fn should_return_401_if_unauthorized() -> Result<()> {
        //let state = mock_state().await;
        //let url_alias = "test";
        //let mut tags = Map::new();
        //tags.insert("url_alias".into(), Value::String(url_alias.into()));
        //Area::insert(GeoJson::Feature(Feature::default()), tags, &state.conn)?;
        //let app = test::init_service(
        //    App::new()
        //        .app_data(Data::new(state.pool))
        //        .service(super::delete),
        //)
        //.await;
        //let req = TestRequest::delete()
        //    .uri(&format!("/{url_alias}"))
        //    .to_request();
        //let res = test::call_service(&app, req).await;
        //assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    async fn delete_should_soft_delete_area() -> Result<()> {
        //let state = mock_state().await;
        //let admin_password = admin::service::mock_admin("test", &state.pool)
        //    .await
        //    .password;
        //let url_alias = "test";
        //let mut tags = Map::new();
        //tags.insert("url_alias".into(), Value::String(url_alias.into()));
        //Area::insert(
        //    GeoJson::from_json_value(phuket_geo_json()).unwrap(),
        //    tags,
        //    &state.conn,
        //)?;
        //let area_element = Element::insert(
        //    &OverpassElement {
        //        lat: Some(7.979623499157051),
        //        lon: Some(98.33448362485439),
        //        ..OverpassElement::mock(1)
        //    },
        //    &state.conn,
        //)?;
        //let area_element = Element::set_tag(
        //    area_element.id,
        //    "areas",
        //    &json!([{"name":"test"}]),
        //    &state.conn,
        //)?;
        //assert!(
        //    area_element
        //        .tags
        //        .get("areas")
        //        .unwrap()
        //        .as_array()
        //        .unwrap()
        //        .len()
        //        == 1
        //);
        //let app = test::init_service(
        //    App::new()
        //        .app_data(Data::new(state.pool))
        //        .service(super::delete),
        //)
        //.await;
        //let req = TestRequest::delete()
        //    .uri(&format!("/{url_alias}"))
        //    .append_header(("Authorization", format!("Bearer {admin_password}")))
        //    .to_request();
        //let res = test::call_service(&app, req).await;
        //assert_eq!(res.status(), StatusCode::OK);
        //let area: Option<Area> = Area::select_by_alias(&url_alias, &state.conn)?;
        //assert!(area.is_some());
        //assert!(area.unwrap().deleted_at.is_some());
        //let area_element = Area::select_by_id(1, &state.conn)?.unwrap();
        //assert!(area_element.tags.get("areas").is_none());
        Ok(())
    }
}
