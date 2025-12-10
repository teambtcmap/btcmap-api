use crate::db::area::schema::Area;
use crate::{service, Result};
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    pub id: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct Res {
    pub id: i64,
    pub tags: JsonObject,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<Area> for Res {
    fn from(val: Area) -> Self {
        Res {
            id: val.id,
            tags: val.tags,
            created_at: val.created_at,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    service::area::soft_delete_async(params.id, pool)
        .await
        .map(Into::into)
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
