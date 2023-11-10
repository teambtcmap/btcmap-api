use crate::{
    area::{Area, AreaRepo},
    auth::AuthService,
    ApiError,
};
use actix_web::{
    delete, patch, post,
    web::{Data, Form, Json, Path},
    HttpRequest, HttpResponse, Responder,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use time::OffsetDateTime;
use tracing::warn;

#[derive(Serialize, Deserialize)]
struct PostArgs {
    tags: HashMap<String, Value>,
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

#[post("")]
async fn post(
    req: HttpRequest,
    args: Json<PostArgs>,
    auth: Data<AuthService>,
    repo: Data<AreaRepo>,
) -> Result<impl Responder, ApiError> {
    auth.check(&req).await?;
    if !args.tags.contains_key("url_alias") {
        Err(ApiError::new(500, format!("url_alias is missing")))?
    }
    let url_alias = &args.tags.get("url_alias").unwrap();
    if !url_alias.is_string() {
        Err(ApiError::new(500, format!("url_alias should be a string")))?
    }
    let url_alias = url_alias.as_str().unwrap();
    if let Some(_) = repo.select_by_url_alias(url_alias).await? {
        Err(ApiError::new(
            303,
            format!("Area with url_alias = {} already exists", url_alias),
        ))?
    }
    repo.insert(&args.tags).await.map_err(|_| {
        ApiError::new(
            500,
            format!("Failed to insert area with url_alias = {}", url_alias),
        )
    })?;
    Ok(Json(json!({
        "message": format!("Area with url_alias = {} has been created", url_alias),
    })))
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
    use std::collections::HashMap;

    use crate::area::admin::PostTagsArgs;
    use crate::area::{Area, AreaRepo};
    use crate::auth::Token;
    use crate::test::mock_state;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use http::StatusCode;
    use serde_json::{json, Value};

    #[test]
    async fn post() -> Result<()> {
        let state = mock_state();
        let token = Token::insert(1, "test", &state.conn)?.secret;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.auth))
                .app_data(Data::new(AreaRepo::new(&state.pool)))
                .service(scope("/").service(super::post)),
        )
        .await;
        let args = r#"
        {
            "tags": {
                "url_alias": "test-area",
                "string": "bar",
                "int": 5,
                "float": 12.34,
                "bool": false
            }
        }
        "#;
        let args: Value = serde_json::from_str(args)?;
        let req = TestRequest::post()
            .uri("/")
            .append_header(("Authorization", format!("Bearer {token}")))
            .set_json(args)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert!(res.status().is_success());
        let area = state
            .area_repo
            .select_by_url_alias("test-area")
            .await?
            .unwrap();
        assert!(area.tags["string"].is_string());
        assert!(area.tags["int"].is_u64());
        assert!(area.tags["float"].is_f64());
        assert!(area.tags["bool"].is_boolean());
        Ok(())
    }

    #[test]
    async fn patch_by_url_alias() -> Result<()> {
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
