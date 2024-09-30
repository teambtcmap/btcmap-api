use super::Element;
use crate::{
    admin::{self},
    discord,
    osm::overpass::OverpassElement,
    Error,
};
use actix_web::{
    patch,
    web::{Data, Json, Path},
    HttpRequest,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{collections::HashMap, sync::Arc};
use time::OffsetDateTime;
use tracing::warn;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ElementView {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub osm_data: Option<OverpassElement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, Value>>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl Into<ElementView> for Element {
    fn into(self) -> ElementView {
        let id = self.overpass_data.btcmap_id();
        let overpass_data = if self.deleted_at.is_none() {
            Some(self.overpass_data)
        } else {
            None
        };
        let tags = if self.deleted_at.is_none() {
            Some(self.tags)
        } else {
            None
        };
        ElementView {
            id: id,
            osm_data: overpass_data,
            tags,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

impl Into<Json<ElementView>> for Element {
    fn into(self) -> Json<ElementView> {
        Json(self.into())
    }
}

#[derive(Serialize, Deserialize)]
struct PatchArgs {
    tags: Map<String, Value>,
}

#[patch("{id}")]
pub async fn patch(
    req: HttpRequest,
    id: Path<String>,
    args: Json<PatchArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Json<ElementView>, Error> {
    let admin = admin::service::check(&req, &pool).await?;
    let cloned_id = id.clone();
    let element = pool
        .get()
        .await?
        .interact(move |conn| Element::select_by_id_or_osm_id(&cloned_id, &conn))
        .await??
        .ok_or(Error::HttpNotFound(format!(
            "There is no area with id or alias = {}",
            id,
        )))?;
    let element = pool
        .get()
        .await?
        .interact(move |conn| Element::patch_tags(element.id, &args.tags.clone(), conn))
        .await??;
    let log_message = format!(
        "{} updated element {} https://api.btcmap.org/v3/elements/{}",
        admin.name,
        element.name(),
        element.id,
    );
    warn!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(element.into())
}

#[cfg(test)]
mod test {
    use crate::element::admin::PatchArgs;
    use crate::element::Element;
    use crate::osm::overpass::OverpassElement;
    use crate::test::mock_state;
    use crate::{admin, Result};
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::Data;
    use actix_web::{test, App};
    use serde_json::{Map, Value};

    #[test]
    async fn patch_unauthorized() -> Result<()> {
        let state = mock_state().await;
        Element::insert(&OverpassElement::mock(1), &state.conn)?;
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
    async fn patch() -> Result<()> {
        let state = mock_state().await;
        let admin_password = admin::service::mock_admin("test", &state.pool)
            .await
            .password;
        let element = Element::insert(&OverpassElement::mock(1), &state.conn)?;
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
            .uri(&format!("/{}", element.overpass_data.btcmap_id()))
            .append_header(("Authorization", format!("Bearer {admin_password}")))
            .set_json(args)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        let element = Element::select_by_id(element.id, &state.conn)?.unwrap();
        assert!(element.tags["string"].is_string());
        assert!(element.tags["unsigned"].is_u64());
        assert!(element.tags["float"].is_f64());
        assert!(element.tags["bool"].is_boolean());
        Ok(())
    }
}
