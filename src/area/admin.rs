use crate::{
    area::{Area, AreaRepo},
    auth::AuthService,
    discord, Error,
};
use actix_web::{
    delete, patch, post,
    web::{Data, Json, Path},
    HttpRequest,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
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
async fn post(
    req: HttpRequest,
    args: Json<PostArgs>,
    auth: Data<AuthService>,
    repo: Data<AreaRepo>,
) -> Result<Json<AreaView>, Error> {
    let token = auth.check(&req).await?;
    let url_alias = &args
        .tags
        .get("url_alias")
        .ok_or(Error::HttpBadRequest(
            "Mandatory tag is missing: url_alias".into(),
        ))?
        .as_str()
        .ok_or(Error::HttpBadRequest(
            "This tag should be a string: url_alias".into(),
        ))?;
    if repo.select_by_url_alias(url_alias).await?.is_some() {
        Err(Error::HttpConflict(
            "This url_alias is already in use".into(),
        ))?
    }
    let area = repo.insert(&args.tags).await?;
    let log_message = format!(
        "{} created a new area: https://api.btcmap.org/v2/areas/{}",
        token.owner,
        area.tags["url_alias"].as_str().unwrap(),
    );
    warn!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area.into())
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
    repo: Data<AreaRepo>,
) -> Result<Json<AreaView>, Error> {
    let token = auth.check(&req).await?;
    let int_id = id.parse::<i64>();
    let area = match int_id {
        Ok(id) => repo.select_by_id(id).await,
        Err(_) => repo.select_by_url_alias(&id).await,
    }?
    .ok_or(Error::HttpNotFound(format!(
        "There is no area with id or url_alias = {}",
        id,
    )))?;
    let area = repo.patch_tags(area.id, &args.tags).await?;
    let log_message = format!(
        "{} updated area https://api.btcmap.org/v2/areas/{}",
        token.owner,
        area.tags["url_alias"].as_str().unwrap(),
    );
    warn!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area.into())
}

#[delete("{id}")]
async fn delete(
    req: HttpRequest,
    id: Path<String>,
    auth: Data<AuthService>,
    repo: Data<AreaRepo>,
) -> Result<Json<AreaView>, Error> {
    let token = auth.check(&req).await?;
    let int_id = id.parse::<i64>();
    let area = match int_id {
        Ok(id) => repo.select_by_id(id).await,
        Err(_) => repo.select_by_url_alias(&id).await,
    }?
    .ok_or(Error::HttpNotFound(format!(
        "There is no area with id or url_alias = {}",
        id,
    )))?;
    let area = repo
        .set_deleted_at(area.id, Some(OffsetDateTime::now_utc()))
        .await?;
    let log_message = format!(
        "User {} deleted area https://api.btcmap.org/v2/areas/{}",
        token.owner,
        area.tags["url_alias"].as_str().unwrap(),
    );
    warn!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
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
mod tests {
    use crate::area::admin::{AreaView, PatchArgs, PostArgs};
    use crate::area::{Area, AreaRepo};
    use crate::test::{mock_state, mock_tags};
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use http::StatusCode;
    use serde_json::{json, Map, Value};

    #[test]
    async fn post_unauthorized() -> Result<()> {
        let state = mock_state().await;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(state.area_repo))
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
    async fn post() -> Result<()> {
        let state = mock_state().await;
        let token = state.auth.mock_token("test").await.secret;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(state.area_repo.clone()))
                .service(scope("/").service(super::post)),
        )
        .await;
        let mut tags = mock_tags();
        let url_alias = json!("test");
        tags.insert("url_alias".into(), url_alias.clone());
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
    async fn patch_unauthorized() -> Result<()> {
        let state = mock_state().await;
        state.area_repo.insert(&Map::new()).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(state.area_repo))
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
        let url_alias = "test";
        let mut tags = Map::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        state.area_repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(AreaRepo::new(&state.pool)))
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
        let area = state
            .area_repo
            .select_by_url_alias(&url_alias)
            .await?
            .unwrap();
        assert!(area.tags["string"].is_string());
        assert!(area.tags["unsigned"].is_u64());
        assert!(area.tags["float"].is_f64());
        assert!(area.tags["bool"].is_boolean());
        Ok(())
    }

    #[test]
    async fn delete_unauthorized() -> Result<()> {
        let state = mock_state().await;
        let url_alias = "test";
        let mut tags = Map::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        state.area_repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(AreaRepo::new(&state.pool)))
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
    async fn delete() -> Result<()> {
        let state = mock_state().await;
        let token = state.auth.mock_token("test").await.secret;
        let url_alias = "test";
        let mut tags = Map::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        state.area_repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(AreaRepo::new(&state.pool)))
                .service(super::delete),
        )
        .await;
        let req = TestRequest::delete()
            .uri(&format!("/{url_alias}"))
            .append_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        let area: Option<Area> = state.area_repo.select_by_url_alias(&url_alias).await?;
        assert!(area.is_some());
        assert!(area.unwrap().deleted_at != None);
        Ok(())
    }
}
