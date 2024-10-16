use crate::{
    admin::{self},
    area::{self, Area},
    Error,
};
use actix_web::{
    delete,
    web::{Data, Json, Path},
    HttpRequest,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::warn;

#[derive(Serialize, Deserialize)]
pub struct AreaView {
    pub id: i64,
    pub tags: Map<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

#[derive(Serialize, Deserialize)]
struct PostArgs {
    tags: Map<String, Value>,
}

#[derive(Serialize, Deserialize)]
struct PatchArgs {
    tags: Map<String, Value>,
}

#[delete("{id_or_alias}")]
pub async fn delete(
    req: HttpRequest,
    id_or_alias: Path<String>,
    pool: Data<Arc<Pool>>,
) -> Result<Json<AreaView>, Error> {
    let admin = admin::service::check(&req, &pool).await?;
    let cloned_id_or_alias = id_or_alias.clone();
    let area = pool
        .get()
        .await?
        .interact(move |conn| Area::select_by_id_or_alias(&cloned_id_or_alias, &conn))
        .await??
        .ok_or(Error::HttpNotFound(format!(
            "There is no area with id or alias = {}",
            id_or_alias,
        )))?;
    let area = pool
        .get()
        .await?
        .interact(move |conn| area::service::soft_delete(&area.id.to_string(), conn))
        .await??;
    let log_message = format!(
        "{} deleted area https://api.btcmap.org/v3/areas/{}",
        admin.name, area.id,
    );
    warn!(log_message);
    Ok(area.into())
}

impl Into<AreaView> for Area {
    fn into(self) -> AreaView {
        AreaView {
            id: self.id,
            tags: self.tags,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

impl Into<Json<AreaView>> for Area {
    fn into(self) -> Json<AreaView> {
        Json(self.into())
    }
}

#[cfg(test)]
mod test {
    use crate::area::Area;
    use crate::element::Element;
    use crate::osm::overpass::OverpassElement;
    use crate::test::{mock_state, phuket_geo_json};
    use crate::{admin, Result};
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::Data;
    use actix_web::{test, App};
    use geojson::{Feature, GeoJson};
    use serde_json::{json, Map, Value};

    #[test]
    async fn delete_should_return_401_if_unauthorized() -> Result<()> {
        let state = mock_state().await;
        let url_alias = "test";
        let mut tags = Map::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        Area::insert(GeoJson::Feature(Feature::default()), tags, &state.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
                .service(super::delete),
        )
        .await;
        let req = TestRequest::delete()
            .uri(&format!("/{url_alias}"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn delete_should_soft_delete_area() -> Result<()> {
        let state = mock_state().await;

        let admin_password = admin::service::mock_admin("test", &state.pool)
            .await
            .password;

        let url_alias = "test";
        let mut tags = Map::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        Area::insert(
            GeoJson::from_json_value(phuket_geo_json()).unwrap(),
            tags,
            &state.conn,
        )?;

        let area_element = Element::insert(
            &OverpassElement {
                lat: Some(7.979623499157051),
                lon: Some(98.33448362485439),
                ..OverpassElement::mock(1)
            },
            &state.conn,
        )?;
        let area_element = Element::set_tag(
            area_element.id,
            "areas",
            &json!([{"name":"test"}]),
            &state.conn,
        )?;

        assert!(
            area_element
                .tags
                .get("areas")
                .unwrap()
                .as_array()
                .unwrap()
                .len()
                == 1
        );

        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
                .service(super::delete),
        )
        .await;
        let req = TestRequest::delete()
            .uri(&format!("/{url_alias}"))
            .append_header(("Authorization", format!("Bearer {admin_password}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);

        let area: Option<Area> = Area::select_by_alias(&url_alias, &state.conn)?;
        assert!(area.is_some());
        assert!(area.unwrap().deleted_at.is_some());

        let area_element = Area::select_by_id(1, &state.conn)?.unwrap();
        assert!(area_element.tags.get("areas").is_none());

        Ok(())
    }
}
