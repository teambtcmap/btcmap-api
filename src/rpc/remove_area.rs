use super::model::RpcArea;
use crate::db::conf::schema::Conf;
use crate::db::user::schema::User;
use crate::service::discord;
use crate::{service, Result};
use deadpool_sqlite::Pool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub id: String,
}

pub async fn run(params: Params, user: &User, pool: &Pool, conf: &Conf) -> Result<RpcArea> {
    let area = service::area::soft_delete_async(params.id, pool).await?;
    discord::send(
        format!("{} removed area {} ({})", user.name, area.name(), area.id,),
        discord::Channel::Api,
        conf,
    );
    Ok(area.into())
}

#[cfg(test)]
mod test {
    use crate::Result;
    use actix_web::test;

    #[test]
    async fn should_return_401_if_unauthorized() -> Result<()> {
        //let state = mock_state().await;
        //let url_alias = "test";
        //let mut tags = Map::new();
        //tags.insert("url_alias".into(), Value::String(url_alias.into()));
        //Area::insert(GeoJson::Feature(Feature::default()), tags, &state.conn)?;
        //let app = test::init_service(
        //    App::new()
        //        .app_data(Data::from(state.pool))
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

    #[test]
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
        //        .app_data(Data::from(state.pool))
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
