use super::Element;
use crate::{
    auth::AuthService, discord, element::ElementRepo, osm::overpass::OverpassElement, Error,
};
use actix_web::{
    patch, post,
    web::{Data, Form, Json, Path},
    HttpRequest,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
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
async fn patch(
    req: HttpRequest,
    id: Path<String>,
    args: Json<PatchArgs>,
    auth: Data<AuthService>,
    repo: Data<ElementRepo>,
) -> Result<Json<ElementView>, Error> {
    let token = auth.check(&req).await?;
    let int_id = id.parse::<i64>();
    let element = match int_id {
        Ok(id) => repo.select_by_id(id).await,
        Err(_) => {
            let id_parts: Vec<&str> = id.split(":").collect();
            if id_parts.len() != 2 {
                Err(Error::HttpBadRequest("Invalid identifier".into()))?
            }
            let r#type = id_parts[0];
            let id = id_parts[1]
                .parse::<i64>()
                .map_err(|_| Error::HttpBadRequest("Invalid identifier".into()))?;
            repo.select_by_osm_type_and_id(r#type, id).await
        }
    }?
    .ok_or(Error::HttpNotFound(format!(
        "There is no element with id = {id}"
    )))?;
    let element = repo.patch_tags(element.id, &args.tags).await?;
    let log_message = format!(
        "{} updated element https://api.btcmap.org/v2/elements/{}",
        token.owner,
        element.overpass_data.btcmap_id(),
    );
    warn!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(element.into())
}

#[derive(Serialize, Deserialize)]
struct PostTagsArgs {
    name: String,
    value: String,
}

#[post("{id}/tags")]
async fn post_tags(
    req: HttpRequest,
    id: Path<String>,
    args: Form<PostTagsArgs>,
    auth: Data<AuthService>,
    repo: Data<ElementRepo>,
) -> Result<Json<ElementView>, Error> {
    let token = auth.check(&req).await?;
    let id_parts: Vec<&str> = id.split(":").collect();
    if id_parts.len() != 2 {
        Err(Error::HttpBadRequest("Invalid identifier".into()))?
    }
    let r#type = id_parts[0];
    let id = id_parts[1]
        .parse::<i64>()
        .map_err(|_| Error::HttpBadRequest("Invalid identifier".into()))?;
    let element = repo
        .select_by_osm_type_and_id(r#type, id)
        .await?
        .ok_or(Error::HttpNotFound(format!(
            "There is no element with id = {}",
            id,
        )))?;
    let element = if args.value.len() > 0 {
        repo.set_tag(element.id, &args.name, &args.value.clone().into())
            .await?
    } else {
        repo.remove_tag(element.id, &args.name).await?
    };
    let log_message = format!(
        "WARNING: {} used DEPRECATED API to set {} = {}",
        token.owner, args.name, args.value,
    );
    warn!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(element.into())
}

#[patch("{id}/tags")]
async fn patch_tags(
    req: HttpRequest,
    id: Path<String>,
    args: Json<Map<String, Value>>,
    auth: Data<AuthService>,
    repo: Data<ElementRepo>,
) -> Result<Json<ElementView>, Error> {
    let token = auth.check(&req).await?;
    let id_parts: Vec<&str> = id.split(":").collect();
    if id_parts.len() != 2 {
        Err(Error::HttpBadRequest("Invalid identifier".into()))?
    }
    let r#type = id_parts[0];
    let id = id_parts[1]
        .parse::<i64>()
        .map_err(|_| Error::HttpBadRequest("Invalid identifier".into()))?;
    let element = repo
        .select_by_osm_type_and_id(r#type, id)
        .await?
        .ok_or(Error::HttpNotFound(format!(
            "There is no element with id = {}",
            id,
        )))?;
    let element = repo.patch_tags(element.id, &args).await?;
    let log_message = format!(
        "{} patched tags for element https://api.btcmap.org/v2/elements/{} {}",
        token.owner,
        id,
        serde_json::to_string_pretty(&args).unwrap(),
    );
    warn!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(element.into())
}

#[cfg(test)]
mod test {
    use crate::element::admin::{PatchArgs, PostTagsArgs};
    use crate::element::ElementRepo;
    use crate::osm::overpass::OverpassElement;
    use crate::test::mock_state;
    use crate::Result;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::Data;
    use actix_web::{test, App};
    use serde_json::{json, Map, Value};

    #[test]
    async fn patch_unauthorized() -> Result<()> {
        let state = mock_state().await;
        state.element_repo.insert(&OverpassElement::mock(1)).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(state.element_repo))
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
        let token = state.auth.mock_token("test").await.secret;
        let element = state.element_repo.insert(&OverpassElement::mock(1)).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(ElementRepo::new(&state.pool)))
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
            .append_header(("Authorization", format!("Bearer {token}")))
            .set_json(args)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        let element = state.element_repo.select_by_id(element.id).await?.unwrap();
        assert!(element.tags["string"].is_string());
        assert!(element.tags["unsigned"].is_u64());
        assert!(element.tags["float"].is_f64());
        assert!(element.tags["bool"].is_boolean());
        Ok(())
    }

    #[test]
    async fn post_tags() -> Result<()> {
        let state = mock_state().await;
        let token = state.auth.mock_token("test").await.secret;
        let element = state.element_repo.insert(&OverpassElement::mock(1)).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(state.element_repo))
                .service(super::post_tags),
        )
        .await;
        let req = TestRequest::post()
            .uri(&format!("/{}/tags", element.overpass_data.btcmap_id()))
            .append_header(("Authorization", format!("Bearer {token}")))
            .set_form(PostTagsArgs {
                name: "foo".into(),
                value: "bar".into(),
            })
            .to_request();
        let res = test::call_service(&app, req).await;
        assert!(res.status().is_success());
        Ok(())
    }

    #[test]
    async fn patch_tags() -> Result<()> {
        let state = mock_state().await;
        let token = state.auth.mock_token("test").await.secret;
        let element = state.element_repo.insert(&OverpassElement::mock(1)).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(state.element_repo))
                .service(super::patch_tags),
        )
        .await;
        let req = TestRequest::patch()
            .uri(&format!("/{}/tags", element.overpass_data.btcmap_id()))
            .append_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "foo": "bar" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        Ok(())
    }
}
