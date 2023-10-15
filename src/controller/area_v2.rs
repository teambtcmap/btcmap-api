use std::collections::HashMap;

use crate::model::Area;
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
use rusqlite::Connection;
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
    id: String,
    tags: HashMap<String, Value>,
}

#[derive(Deserialize)]
pub struct GetArgs {
    updated_since: Option<String>,
    limit: Option<i32>,
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
            id: self.tag("url_alias").as_str().unwrap_or_default().into(),
            tags: self.tags.unwrap_or_default(),
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

#[derive(Serialize, Deserialize)]
struct PatchArgs {
    tags: Value,
}

#[derive(Serialize, Deserialize)]
struct PostTagsArgs {
    name: String,
    value: String,
}

#[post("")]
async fn post_json(
    args: Json<PostJsonArgs>,
    req: HttpRequest,
    conn: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;

    warn!(
        user_id = token.user_id,
        area_url = args.id,
        "User attempted to create an area",
    );

    if let Some(_) = Area::select_by_url_alias(&args.id, &conn)? {
        Err(ApiError::new(
            303,
            format!("Area with url = {} already exists", args.id),
        ))?
    }

    if let Err(_) = Area::insert_or_replace(&args.id, Some(&args.tags), &conn) {
        Err(ApiError::new(
            500,
            format!("Failed to insert area with url = {}", args.id),
        ))?
    }

    Ok(Json(json!({
        "message": format!("Area with url = {} has been created", args.id),
    })))
}

#[get("")]
async fn get(args: Query<GetArgs>, conn: Data<Connection>) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => Area::select_updated_since(&conn, updated_since, args.limit)?
            .into_iter()
            .map(|it| it.into())
            .collect(),
        None => Area::select_all(&conn, args.limit)?
            .into_iter()
            .map(|it| it.into())
            .collect(),
    }))
}

#[get("{url_alias}")]
async fn get_by_url_alias(
    url_alias: Path<String>,
    conn: Data<Connection>,
) -> Result<Json<GetItem>, ApiError> {
    Area::select_by_url_alias(&url_alias, &conn)?
        .map(|it| Json(it.into()))
        .ok_or(ApiError::new(
            404,
            &format!("Area with url_alias = {url_alias} doesn't exist"),
        ))
}

#[patch("{url_alias}")]
async fn patch_by_url_alias(
    url_alias: Path<String>,
    req: HttpRequest,
    args: Json<PatchArgs>,
    conn: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;
    let area_url_alias = url_alias.into_inner();

    warn!(
        token.user_id,
        area_url_alias, "User attempted to update an area",
    );

    let area: Option<Area> = Area::select_by_url_alias(&area_url_alias, &conn)?;

    let area = match area {
        Some(v) => v,
        None => {
            return Err(ApiError::new(
                404,
                &format!("There is no area with url_alias = {area_url_alias}"),
            ));
        }
    };

    let new_tags = match args.tags.clone() {
        Value::Object(v) => v,
        _ => {
            return Err(ApiError::new(
                400,
                &format!("The field tags should be an object"),
            ));
        }
    };

    let mut merged_tags = area.tags.clone().unwrap_or_default();

    for (new_key, new_value) in &new_tags {
        if merged_tags.contains_key(new_key) {
            warn!(
                token.user_id,
                area_url_alias,
                tag = new_key,
                old_value = serde_json::to_string(&merged_tags[new_key]).unwrap(),
                new_value = serde_json::to_string(new_value).unwrap(),
                "Admin user updated an existing tag",
            );
        } else {
            warn!(
                token.user_id,
                area_url_alias,
                tag_name = new_key,
                tag_value = serde_json::to_string(new_value).unwrap(),
                "Admin user added new tag",
            );
        }

        merged_tags.insert(new_key.clone(), new_value.clone());
    }

    Area::insert_or_replace(&area_url_alias, Some(&merged_tags), &conn)?;

    Ok(HttpResponse::Ok())
}

#[patch("{url_alias}/tags")]
async fn patch_tags(
    args: Json<HashMap<String, Value>>,
    conn: Data<Connection>,
    url_alias: Path<String>,
    req: HttpRequest,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;
    let area_url_alias = url_alias.into_inner();

    let area: Option<Area> = Area::select_by_url_alias(&area_url_alias, &conn)?;

    let area = match area {
        Some(v) => v,
        None => {
            return Err(ApiError::new(
                404,
                &format!("There is no area with url = {area_url_alias}"),
            ));
        }
    };

    let mut merged_tags = area.tags.clone().unwrap_or_default();

    for (new_key, new_value) in &args.0 {
        if merged_tags.contains_key(new_key) {
            warn!(
                token.user_id,
                area_url_alias,
                tag = new_key,
                old_value = serde_json::to_string(&merged_tags[new_key]).unwrap(),
                new_value = serde_json::to_string(new_value).unwrap(),
                "Admin user updated an existing tag",
            );
        } else {
            warn!(
                token.user_id,
                area_url_alias,
                tag_name = new_key,
                tag_value = serde_json::to_string(new_value).unwrap(),
                "Admin user added new tag",
            );
        }

        merged_tags.insert(new_key.clone(), new_value.clone());
    }

    Area::insert_or_replace(&area_url_alias, Some(&merged_tags), &conn)?;

    Ok(HttpResponse::Ok())
}

#[post("{url_alias}/tags")]
async fn post_tags(
    url_alias: Path<String>,
    req: HttpRequest,
    args: Form<PostTagsArgs>,
    conn: Data<Connection>,
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

    let area: Option<Area> = Area::select_by_url_alias(&area_url_alias, &conn)?;

    match area {
        Some(area) => {
            if args.value.len() > 0 {
                area.insert_tag(&args.name, &args.value, &conn)?;
            } else {
                area.delete_tag(&args.name, &conn)?;
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
    url_alias: Path<String>,
    req: HttpRequest,
    conn: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;
    let url_alias = url_alias.into_inner();

    warn!(token.user_id, url_alias, "User attempted to delete an area",);

    let area: Option<Area> = Area::select_by_url_alias(&url_alias, &conn)?;

    match area {
        Some(area) => {
            area.delete(&conn)?;
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
    use crate::command::db;
    use crate::model::token;
    use crate::Result;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};
    use rusqlite::named_params;
    use tracing::info;

    #[actix_web::test]
    async fn post_json() -> Result<()> {
        let db_path = "file:area_v2_post_json?mode=memory&cache=shared";
        let mut conn = Connection::open(db_path)?;
        db::migrate(&mut conn)?;

        let admin_token = "test";
        conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(scope("/").service(super::post_json)),
        )
        .await;

        let args = r#"
        {
            "id": "test-area",
            "tags": {
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
        info!(response_status = ?res.status());
        assert!(res.status().is_success());

        let area = Area::select_by_url_alias("test-area", &Connection::open(db_path)?)?.unwrap();

        assert!(area.tags.as_ref().unwrap()["string"].is_string());
        assert!(area.tags.as_ref().unwrap()["int"].is_u64());
        assert!(area.tags.as_ref().unwrap()["float"].is_f64());
        assert!(area.tags.as_ref().unwrap()["bool"].is_boolean());

        Ok(())
    }

    #[actix_web::test]
    async fn get_empty_table() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 0);
        Ok(())
    }

    #[actix_web::test]
    async fn get_one_row() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        Area::insert_or_replace("test", None, &conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(scope("/").service(super::get)),
        )
        .await;

        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 1);

        Ok(())
    }

    #[actix_web::test]
    async fn get_with_limit() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        Area::insert_or_replace("test1", None, &conn)?;
        Area::insert_or_replace("test2", None, &conn)?;
        Area::insert_or_replace("test3", None, &conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(scope("/").service(super::get)),
        )
        .await;

        let req = TestRequest::get().uri("/?limit=2").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 2);

        Ok(())
    }

    #[actix_web::test]
    async fn get_by_id() -> Result<()> {
        let conn = db::setup_connection()?;
        let area_url_alias = "test";
        Area::insert_or_replace(&area_url_alias, None, &conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
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

    #[actix_web::test]
    async fn patch_tags() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        let admin_token = "test";
        conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;

        let area_url = "test";
        Area::insert_or_replace(&area_url, None, &conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::patch_tags),
        )
        .await;
        let req = TestRequest::patch()
            .uri(&format!("/{area_url}/tags"))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_json(json!({ "foo": "bar" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        Ok(())
    }

    #[actix_web::test]
    async fn patch_by_id() -> Result<()> {
        let db_path = "file:area_v2_patch_by_id?mode=memory&cache=shared";
        let mut conn = Connection::open(db_path)?;
        db::migrate(&mut conn)?;

        let admin_token = "test";
        conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;

        let area_url = "test";
        Area::insert_or_replace(&area_url, None, &conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
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
            .uri(&format!("/{area_url}"))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_json(args)
            .to_request();

        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);

        let area = Area::select_by_url_alias(&area_url, &Connection::open(db_path)?)?.unwrap();

        assert!(area.tags.as_ref().unwrap()["string"].is_string());
        assert!(area.tags.as_ref().unwrap()["unsigned"].is_u64());
        assert!(area.tags.as_ref().unwrap()["float"].is_f64());
        assert!(area.tags.as_ref().unwrap()["bool"].is_boolean());

        Ok(())
    }

    #[actix_web::test]
    async fn post_tags() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        let admin_token = "test";
        conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;

        let area_url = "test";
        Area::insert_or_replace(&area_url, None, &conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::post_tags),
        )
        .await;

        let req = TestRequest::post()
            .uri(&format!("/{area_url}/tags"))
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
        let db_path = "file:area_v2_delete?mode=memory&cache=shared";
        let mut conn = Connection::open(db_path)?;
        db::migrate(&mut conn)?;

        let admin_token = "test";
        conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;

        let area_url_alias = "test";
        Area::insert_or_replace(&area_url_alias, None, &conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::delete_by_url_alias),
        )
        .await;
        let req = TestRequest::delete()
            .uri(&format!("/{area_url_alias}"))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);

        let area: Option<Area> =
            Area::select_by_url_alias(&area_url_alias, &Connection::open(db_path)?)?;

        assert!(area.is_some());

        assert!(area.unwrap().deleted_at != None);

        Ok(())
    }
}
