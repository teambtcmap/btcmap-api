use super::Element;
use crate::{auth::AuthService, element::ElementRepo, osm::overpass::OverpassElement, ApiError};
use actix_web::{
    patch, post,
    web::{Data, Form, Json, Path},
    HttpRequest,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use time::OffsetDateTime;
use tracing::debug;

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
) -> Result<Json<ElementView>, ApiError> {
    let token = auth.check(&req).await?;
    let id_parts: Vec<&str> = id.split(":").collect();
    if id_parts.len() != 2 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "Invalid identifier"));
    }
    let r#type = id_parts[0];
    let id = id_parts[1]
        .parse::<i64>()
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "Invalid identifier"))?;
    let element = repo
        .select_by_osm_type_and_id(r#type, id)
        .await?
        .ok_or(ApiError::new(
            StatusCode::NOT_FOUND,
            &format!("There is no element with id = {}", id),
        ))?;
    let element = if args.value.len() > 0 {
        repo.set_tag(element.id, &args.name, &args.value.clone().into())
            .await?
    } else {
        repo.remove_tag(element.id, &args.name).await?
    };
    debug!(
        admin_channel_message = format!(
            "WARNING: User https://api.btcmap.org/v2/users/{} used DEPRECATED API to set {} = {}",
            token.user_id, args.name, args.value,
        )
    );
    Ok(element.into())
}

#[patch("{id}/tags")]
async fn patch_tags(
    req: HttpRequest,
    id: Path<String>,
    args: Json<HashMap<String, Value>>,
    auth: Data<AuthService>,
    repo: Data<ElementRepo>,
) -> Result<Json<ElementView>, ApiError> {
    let token = auth.check(&req).await?;
    let id_parts: Vec<&str> = id.split(":").collect();
    if id_parts.len() != 2 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "Invalid identifier"));
    }
    let r#type = id_parts[0];
    let id = id_parts[1]
        .parse::<i64>()
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "Invalid identifier"))?;
    let element = repo
        .select_by_osm_type_and_id(r#type, id)
        .await?
        .ok_or(ApiError::new(
            StatusCode::NOT_FOUND,
            &format!("There is no element with id = {}", id),
        ))?;
    let element = repo.patch_tags(element.id, &args).await?;
    debug!(
        admin_channel_message = format!(
            "User https://api.btcmap.org/v2/users/{} patched tags for element https://api.btcmap.org/v2/elements/{} {}",
            token.user_id, id, serde_json::to_string_pretty(&args).unwrap(),
        )
    );
    Ok(element.into())
}

#[cfg(test)]
mod test {
    use crate::auth::Token;
    use crate::element::admin::PostTagsArgs;
    use crate::osm::osm::OsmUser;
    use crate::osm::overpass::OverpassElement;
    use crate::test::mock_state;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::Data;
    use actix_web::{test, App};
    use reqwest::StatusCode;
    use serde_json::json;

    #[test]
    async fn post_tags() -> Result<()> {
        let state = mock_state();
        state.user_repo.insert(1, &OsmUser::mock()).await?;
        let token = Token::insert(1, "test", &state.conn)?.secret;
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
        let state = mock_state();
        state.user_repo.insert(1, &OsmUser::mock()).await?;
        let token = Token::insert(1, "test", &state.conn)?.secret;
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
