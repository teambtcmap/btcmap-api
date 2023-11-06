use std::collections::HashMap;

use crate::model::Area;
use crate::repo::area::AreaRepo;
use crate::service::auth::get_admin_token;
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
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::warn;

#[derive(Serialize, Deserialize)]
struct PostArgs {
    id: String,
}

#[derive(Serialize, Deserialize)]
struct PostJsonArgs {
    tags: HashMap<String, Value>,
}

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

#[post("")]
async fn post_json(
    req: HttpRequest,
    args: Json<PostJsonArgs>,
    conn: Data<PooledConnection<SqliteConnectionManager>>,
    repo: Data<AreaRepo>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;

    if !args.tags.contains_key("url_alias") {
        Err(ApiError::new(500, format!("url_alias is missing")))?
    }

    let url_alias = &args.tags.get("url_alias").unwrap();

    if !url_alias.is_string() {
        Err(ApiError::new(500, format!("url_alias should be a string")))?
    }

    let url_alias = url_alias.as_str().unwrap();

    warn!(token.user_id, url_alias, "User attempted to create an area",);

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
    conn: Data<PooledConnection<SqliteConnectionManager>>,
    repo: Data<AreaRepo>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;
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
    conn: Data<PooledConnection<SqliteConnectionManager>>,
    repo: Data<AreaRepo>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;

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
    conn: Data<PooledConnection<SqliteConnectionManager>>,
    repo: Data<AreaRepo>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;
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
    conn: Data<PooledConnection<SqliteConnectionManager>>,
    repo: Data<AreaRepo>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;
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
    use std::sync::Arc;

    use super::*;
    use crate::model::token;
    use crate::test::{mock_area_repo, mock_conn_pool};
    use crate::Result;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};
    use rusqlite::named_params;

    #[test]
    async fn post_json() -> Result<()> {
        let pool = Arc::new(mock_conn_pool());
        let repo = AreaRepo::new(pool.clone());
        let conn = pool.get()?;
        let admin_token = "test";
        conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .app_data(Data::new(AreaRepo::new(pool.clone())))
                .service(scope("/").service(super::post_json)),
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
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_json(args)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert!(res.status().is_success());
        let area = repo.select_by_url_alias("test-area").await?.unwrap();
        assert!(area.tags["string"].is_string());
        assert!(area.tags["int"].is_u64());
        assert!(area.tags["float"].is_f64());
        assert!(area.tags["bool"].is_boolean());
        Ok(())
    }

    #[test]
    async fn get_empty_table() -> Result<()> {
        let repo = mock_area_repo();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(repo))
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
        let repo = mock_area_repo();
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), "test".into());
        repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(repo))
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
        let repo = mock_area_repo();
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), "test".into());
        repo.insert(&tags).await?;
        repo.insert(&tags).await?;
        repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(repo))
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
        let repo = mock_area_repo();
        let area_url_alias = "test";
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), Value::String(area_url_alias.into()));
        repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(repo))
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
        let pool = Arc::new(mock_conn_pool());
        let repo = AreaRepo::new(pool.clone());
        let conn = pool.get()?;
        let admin_token = "test";
        conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        let url_alias = "test";
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .app_data(Data::new(repo))
                .service(super::patch_tags),
        )
        .await;
        let req = TestRequest::patch()
            .uri(&format!("/{url_alias}/tags"))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_json(json!({ "foo": "bar" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        Ok(())
    }

    #[test]
    async fn patch_by_id() -> Result<()> {
        let pool = Arc::new(mock_conn_pool());
        let repo = AreaRepo::new(pool.clone());
        let conn = pool.get()?;
        let admin_token = "test";
        conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        let url_alias = "test";
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .app_data(Data::new(AreaRepo::new(pool.clone())))
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
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_json(args)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        let area = repo.select_by_url_alias(&url_alias).await?.unwrap();
        assert!(area.tags["string"].is_string());
        assert!(area.tags["unsigned"].is_u64());
        assert!(area.tags["float"].is_f64());
        assert!(area.tags["bool"].is_boolean());
        Ok(())
    }

    #[test]
    async fn post_tags() -> Result<()> {
        let pool = Arc::new(mock_conn_pool());
        let repo = AreaRepo::new(pool.clone());
        let conn = pool.get()?;
        let admin_token = "test";
        conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        let url_alias = "test";
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .app_data(Data::new(repo))
                .service(super::post_tags),
        )
        .await;
        let req = TestRequest::post()
            .uri(&format!("/{url_alias}/tags"))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_form(PostTagsArgs {
                name: "foo".into(),
                value: "bar".into(),
            })
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::CREATED);
        Ok(())
    }

    #[actix_web::test]
    async fn delete() -> Result<()> {
        let pool = Arc::new(mock_conn_pool());
        let repo = AreaRepo::new(pool.clone());
        let conn = pool.get()?;
        let admin_token = "test";
        conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        let url_alias = "test";
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        repo.insert(&tags).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .app_data(Data::new(AreaRepo::new(pool.clone())))
                .service(super::delete_by_url_alias),
        )
        .await;
        let req = TestRequest::delete()
            .uri(&format!("/{url_alias}"))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        let area: Option<Area> = repo.select_by_url_alias(&url_alias).await?;
        assert!(area.is_some());
        assert!(area.unwrap().deleted_at != None);
        Ok(())
    }
}
