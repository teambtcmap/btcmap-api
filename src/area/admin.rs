use crate::{
    area::{self, Area},
    auth::{self},
    discord, Error,
};
use actix_web::{
    delete, patch, post,
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

#[post("")]
pub async fn post(
    req: HttpRequest,
    args: Json<PostArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Json<AreaView>, Error> {
    let token = auth::service::check(&req, &pool).await?;
    let area = pool
        .get()
        .await?
        .interact(move |conn| area::service::insert(args.tags.clone(), conn))
        .await??;
    let log_message = format!(
        "{} created a new area: https://api.btcmap.org/v3/areas/{}",
        token.owner, area.id,
    );
    warn!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area.into())
}

#[derive(Serialize, Deserialize)]
struct PatchArgs {
    tags: Map<String, Value>,
}

#[patch("{id_or_alias}")]
pub async fn patch(
    req: HttpRequest,
    id_or_alias: Path<String>,
    args: Json<PatchArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Json<AreaView>, Error> {
    let token = auth::service::check(&req, &pool).await?;
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
        .interact(move |conn| {
            area::service::patch_tags(&area.id.to_string(), args.tags.clone(), conn)
        })
        .await??;
    let log_message = format!(
        "{} updated area https://api.btcmap.org/v3/areas/{}",
        token.owner, area.id,
    );
    warn!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area.into())
}

#[delete("{id_or_alias}")]
pub async fn delete(
    req: HttpRequest,
    id_or_alias: Path<String>,
    pool: Data<Arc<Pool>>,
) -> Result<Json<AreaView>, Error> {
    let token = auth::service::check(&req, &pool).await?;
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
        token.owner, area.id,
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
    use crate::area::admin::{AreaView, PatchArgs, PostArgs};
    use crate::area::Area;
    use crate::element::Element;
    use crate::osm::overpass::OverpassElement;
    use crate::test::{mock_state, mock_tags, phuket_geo_json};
    use crate::{auth, Result};
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use serde_json::{json, Map, Value};

    #[test]
    async fn post_should_return_401_if_unauthorized() -> Result<()> {
        let state = mock_state().await;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
                .service(scope("/").service(super::post)),
        )
        .await;
        let req = TestRequest::post()
            .uri("/")
            .set_json(json!({"tags": {}}))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn post_should_create_area() -> Result<()> {
        let state = mock_state().await;
        let token = auth::service::mock_token("test", &state.pool).await.secret;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
                .service(scope("/").service(super::post)),
        )
        .await;
        let mut tags = mock_tags();
        let url_alias = json!("test");
        tags.insert("url_alias".into(), url_alias.clone());
        tags.insert("geo_json".into(), phuket_geo_json());
        let req = TestRequest::post()
            .uri("/")
            .append_header(("Authorization", format!("Bearer {token}")))
            .set_json(PostArgs { tags: tags.clone() })
            .to_request();
        let res: AreaView = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.id);
        assert_eq!(tags, res.tags);
        assert!(res.deleted_at.is_none());
        Ok(())
    }

    #[test]
    async fn patch_should_return_401_if_unauthorized() -> Result<()> {
        let state = mock_state().await;
        Area::insert(Map::new(), &state.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
                .service(super::patch),
        )
        .await;
        let req = TestRequest::patch()
            .uri("/1")
            .set_json(PatchArgs { tags: Map::new() })
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn patch_should_update_area() -> Result<()> {
        let state = mock_state().await;
        let token = auth::service::mock_token("test", &state.pool).await.secret;
        let url_alias = "test";
        let mut tags = Map::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        Area::insert(tags, &state.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
                .service(super::patch),
        )
        .await;
        let args = r#"
        {
            "tags": {
                "string": "bar",
                "unsigned": 5,
                "float": 12.34,
                "bool": true
            }
        }
        "#;
        let args: Value = serde_json::from_str(args)?;
        let req = TestRequest::patch()
            .uri(&format!("/{url_alias}"))
            .append_header(("Authorization", format!("Bearer {token}")))
            .set_json(args)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        let area = Area::select_by_alias(&url_alias, &state.conn)?.unwrap();
        assert!(area.tags["string"].is_string());
        assert!(area.tags["unsigned"].is_u64());
        assert!(area.tags["float"].is_f64());
        assert!(area.tags["bool"].is_boolean());
        Ok(())
    }

    #[test]
    async fn delete_should_return_401_if_unauthorized() -> Result<()> {
        let state = mock_state().await;
        let url_alias = "test";
        let mut tags = Map::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        Area::insert(tags, &state.conn)?;
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

        let token = auth::service::mock_token("test", &state.pool).await.secret;

        let url_alias = "test";
        let mut tags = Map::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        tags.insert("geo_json".into(), phuket_geo_json());
        Area::insert(tags, &state.conn)?;

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
            .append_header(("Authorization", format!("Bearer {token}")))
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
