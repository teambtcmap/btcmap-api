use crate::area::Area;
use crate::area::AreaRepo;
use crate::auth::AuthService;
use crate::ApiError;
use actix_web::delete;
use actix_web::get;
use actix_web::patch;
use actix_web::post;
use actix_web::web::Data;
use actix_web::web::Form;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::Responder;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::warn;

#[derive(Deserialize)]
pub struct GetArgs {
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    updated_since: Option<OffsetDateTime>,
    limit: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct GetItem {
    pub id: String,
    pub tags: HashMap<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub deleted_at: String,
}

impl Into<GetItem> for Area {
    fn into(self) -> GetItem {
        GetItem {
            id: self.tags["url_alias"].as_str().unwrap().into(),
            tags: self.tags,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self
                .deleted_at
                .map(|it| it.format(&Rfc3339).unwrap())
                .unwrap_or_default()
                .into(),
        }
    }
}

impl Into<Json<GetItem>> for Area {
    fn into(self) -> Json<GetItem> {
        Json(self.into())
    }
}

#[derive(Serialize, Deserialize)]
struct PatchArgs {
    tags: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize)]
struct PostTagsArgs {
    name: String,
    value: String,
}

#[get("")]
async fn get(args: Query<GetArgs>, repo: Data<AreaRepo>) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => repo
            .select_updated_since(updated_since, args.limit)
            .await?
            .into_iter()
            .map(|it| it.into())
            .collect(),
        None => repo
            .select_all(args.limit)
            .await?
            .into_iter()
            .map(|it| it.into())
            .collect(),
    }))
}

#[get("{url_alias}")]
async fn get_by_url_alias(
    url_alias: Path<String>,
    repo: Data<AreaRepo>,
) -> Result<Json<GetItem>, ApiError> {
    repo.select_by_url_alias(&url_alias)
        .await?
        .ok_or(ApiError::new(
            404,
            &format!("Area with url_alias = {url_alias} doesn't exist"),
        ))
        .map(|it| it.into())
}

#[patch("{url_alias}")]
async fn patch_by_url_alias(
    req: HttpRequest,
    args: Json<PatchArgs>,
    url_alias: Path<String>,
    auth: Data<AuthService>,
    repo: Data<AreaRepo>,
) -> Result<impl Responder, ApiError> {
    let token = auth.check(&req).await?;
    let area_url_alias = url_alias.into_inner();

    warn!(
        token.user_id,
        area_url_alias, "User attempted to merge new tags",
    );

    match repo.select_by_url_alias(&area_url_alias).await? {
        Some(area) => repo.patch_tags(area.id, &args.tags).await?,
        None => {
            return Err(ApiError::new(
                404,
                &format!("There is no area with url_alias = {area_url_alias}"),
            ));
        }
    };

    Ok(HttpResponse::Ok())
}

#[patch("{url_alias}/tags")]
async fn patch_tags(
    req: HttpRequest,
    args: Json<HashMap<String, Value>>,
    url_alias: Path<String>,
    auth: Data<AuthService>,
    repo: Data<AreaRepo>,
) -> Result<impl Responder, ApiError> {
    let token = auth.check(&req).await?;

    warn!(
        token.user_id,
        url_alias = url_alias.as_str(),
        "User attempted to merge new tags",
    );

    match repo.select_by_url_alias(&url_alias).await? {
        Some(area) => repo.patch_tags(area.id, &args).await?,
        None => {
            return Err(ApiError::new(
                404,
                &format!("There is no area with url_alias = {url_alias}"),
            ));
        }
    };

    Ok(HttpResponse::Ok())
}

#[post("{url_alias}/tags")]
async fn post_tags(
    req: HttpRequest,
    args: Form<PostTagsArgs>,
    url_alias: Path<String>,
    auth: Data<AuthService>,
    repo: Data<AreaRepo>,
) -> Result<impl Responder, ApiError> {
    let token = auth.check(&req).await?;
    let area_url_alias = url_alias.into_inner();

    warn!(
        deprecated_api = true,
        token.user_id,
        area_url_alias,
        tag_name = args.name,
        tag_value = args.value,
        "User attempted to update area tag",
    );

    let area: Option<Area> = repo.select_by_url_alias(&area_url_alias).await?;

    match area {
        Some(area) => {
            if args.value.len() > 0 {
                let mut patch_set = HashMap::new();
                patch_set.insert(args.name.clone(), Value::String(args.value.clone()));
                repo.patch_tags(area.id, &patch_set).await?;
            } else {
                repo.remove_tag(area.id, &args.name).await?;
            }

            Ok(HttpResponse::Created())
        }
        None => Err(ApiError::new(
            404,
            &format!("There is no area with url_alias = {area_url_alias}"),
        )),
    }
}

#[delete("{url_alias}")]
async fn delete_by_url_alias(
    req: HttpRequest,
    url_alias: Path<String>,
    auth: Data<AuthService>,
    repo: Data<AreaRepo>,
) -> Result<impl Responder, ApiError> {
    let token = auth.check(&req).await?;
    let url_alias = url_alias.into_inner();

    warn!(token.user_id, url_alias, "User attempted to delete an area",);

    let area: Option<Area> = repo.select_by_url_alias(&url_alias).await?;

    match area {
        Some(area) => {
            repo.set_deleted_at(area.id, Some(OffsetDateTime::now_utc()))
                .await?;
            Ok(HttpResponse::Ok())
        }
        None => Err(ApiError::new(
            404,
            &format!("There is no area with url_alias = {url_alias}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::Token;
    use crate::test::mock_state;
    use crate::Result;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};
    use serde_json::json;

    #[test]
    async fn get_empty_table() -> Result<()> {
        let state = mock_state();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(AreaRepo::new(&state.pool)))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_one_row() -> Result<()> {
        let state = mock_state();
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(AreaRepo::new(&state.pool)))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 1);
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let state = mock_state();
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&tags).await?;
        state.area_repo.insert(&tags).await?;
        state.area_repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(AreaRepo::new(&state.pool)))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=2").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 2);
        Ok(())
    }

    #[test]
    async fn get_by_id() -> Result<()> {
        let state = mock_state();
        let area_url_alias = "test";
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), Value::String(area_url_alias.into()));
        state.area_repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(AreaRepo::new(&state.pool)))
                .service(super::get_by_url_alias),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{area_url_alias}"))
            .to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, area_url_alias);
        Ok(())
    }

    #[test]
    async fn patch_tags() -> Result<()> {
        let state = mock_state();
        let token = Token::insert(1, "test", &state.conn)?.secret;
        let url_alias = "test";
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        state.area_repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(AreaRepo::new(&state.pool)))
                .service(super::patch_tags),
        )
        .await;
        let req = TestRequest::patch()
            .uri(&format!("/{url_alias}/tags"))
            .append_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "foo": "bar" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        Ok(())
    }

    #[test]
    async fn patch_by_id() -> Result<()> {
        let state = mock_state();
        let token = Token::insert(1, "test", &state.conn)?.secret;
        let url_alias = "test";
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        state.area_repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(AreaRepo::new(&state.pool)))
                .service(super::patch_by_url_alias),
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
    async fn post_tags() -> Result<()> {
        let state = mock_state();
        let token = Token::insert(1, "test", &state.conn)?.secret;
        let url_alias = "test";
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        state.area_repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(AreaRepo::new(&state.pool)))
                .service(super::post_tags),
        )
        .await;
        let req = TestRequest::post()
            .uri(&format!("/{url_alias}/tags"))
            .append_header(("Authorization", format!("Bearer {token}")))
            .set_form(PostTagsArgs {
                name: "foo".into(),
                value: "bar".into(),
            })
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::CREATED);
        Ok(())
    }

    #[test]
    async fn delete() -> Result<()> {
        let state = mock_state();
        let token = Token::insert(1, "test", &state.conn)?.secret;
        let url_alias = "test";
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        state.area_repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(AreaRepo::new(&state.pool)))
                .service(super::delete_by_url_alias),
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
