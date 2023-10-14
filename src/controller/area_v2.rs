use std::collections::HashMap;

use crate::model::area;
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
use rusqlite::named_params;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use serde_json::Value;
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
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

impl Into<GetItem> for Area {
    fn into(self) -> GetItem {
        GetItem {
            id: self.id,
            tags: self.tags,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
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
        area_id = args.id,
        "User attempted to create an area",
    );

    if let Some(_) = Area::select_by_id(&args.id, &conn)? {
        Err(ApiError::new(
            303,
            format!("Area {} already exists", args.id),
        ))?
    }

    if let Err(_) = Area::insert_or_replace(&args.id, &args.tags, &conn) {
        Err(ApiError::new(
            500,
            format!("Failed to insert area {}", args.id),
        ))?
    }

    Ok(Json(json!({
        "message": format!("Area {} has been created", args.id),
    })))
}

#[get("")]
async fn get(args: Query<GetArgs>, conn: Data<Connection>) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => conn
            .prepare(area::SELECT_UPDATED_SINCE)?
            .query_map(
                named_params! {
                    ":updated_since": updated_since,
                    ":limit": args.limit.unwrap_or(std::i32::MAX),
                },
                area::SELECT_UPDATED_SINCE_MAPPER,
            )?
            .map(|it| it.map(|it| it.into()))
            .collect::<Result<_, _>>()?,
        None => conn
            .prepare(area::SELECT_ALL)?
            .query_map(
                named_params! { ":limit": args.limit.unwrap_or(std::i32::MAX) },
                area::SELECT_ALL_MAPPER,
            )?
            .map(|it| it.map(|it| it.into()))
            .collect::<Result<_, _>>()?,
    }))
}

#[get("{id}")]
async fn get_by_id(id: Path<String>, conn: Data<Connection>) -> Result<Json<GetItem>, ApiError> {
    Area::select_by_id(&id, &conn)?
        .map(|it| Json(it.into()))
        .ok_or(ApiError::new(
            404,
            &format!("Area with id {id} doesn't exist"),
        ))
}

#[patch("{id}")]
async fn patch_by_id(
    id: Path<String>,
    req: HttpRequest,
    args: Json<PatchArgs>,
    conn: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;
    let area_id = id.into_inner();

    warn!(
        user_id = token.user_id,
        area_id, "User attempted to update an area",
    );

    let area: Option<Area> = Area::select_by_id(&area_id, &conn)?;

    let area = match area {
        Some(v) => v,
        None => {
            return Err(ApiError::new(
                404,
                &format!("There is no area with id {area_id}"),
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

    let mut merged_tags = area.tags.clone();

    for (new_key, new_value) in &new_tags {
        if merged_tags.contains_key(new_key) {
            warn!(
                user_id = token.user_id,
                area_id,
                tag = new_key,
                old_value = serde_json::to_string(&merged_tags[new_key]).unwrap(),
                new_value = serde_json::to_string(new_value).unwrap(),
                "Admin user updated an existing tag",
            );
        } else {
            warn!(
                user_id = token.user_id,
                area_id,
                tag_name = new_key,
                tag_value = serde_json::to_string(new_value).unwrap(),
                "Admin user added new tag",
            );
        }

        merged_tags.insert(new_key.clone(), new_value.clone());
    }

    conn.execute(
        area::UPDATE_TAGS,
        named_params! {
            ":area_id": area.id,
            ":tags": serde_json::to_string(&merged_tags).unwrap(),
        },
    )?;

    Ok(HttpResponse::Ok())
}

#[patch("{id}/tags")]
async fn patch_tags(
    args: Json<HashMap<String, Value>>,
    conn: Data<Connection>,
    id: Path<String>,
    req: HttpRequest,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;
    let area_id = id.into_inner();

    let area: Option<Area> = Area::select_by_id(&area_id, &conn)?;

    let area = match area {
        Some(v) => v,
        None => {
            return Err(ApiError::new(
                404,
                &format!("There is no area with id {area_id}"),
            ));
        }
    };

    let mut merged_tags = area.tags.clone();

    for (new_key, new_value) in &args.0 {
        if merged_tags.contains_key(new_key) {
            warn!(
                user_id = token.user_id,
                area_id,
                tag = new_key,
                old_value = serde_json::to_string(&merged_tags[new_key]).unwrap(),
                new_value = serde_json::to_string(new_value).unwrap(),
                "Admin user updated an existing tag",
            );
        } else {
            warn!(
                user_id = token.user_id,
                area_id,
                tag_name = new_key,
                tag_value = serde_json::to_string(new_value).unwrap(),
                "Admin user added new tag",
            );
        }

        merged_tags.insert(new_key.clone(), new_value.clone());
    }

    conn.execute(
        area::UPDATE_TAGS,
        named_params! {
            ":area_id": area_id,
            ":tags": serde_json::to_string(&merged_tags).unwrap(),
        },
    )?;

    Ok(HttpResponse::Ok())
}

#[post("{id}/tags")]
async fn post_tags(
    id: Path<String>,
    req: HttpRequest,
    args: Form<PostTagsArgs>,
    conn: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;
    let area_id = id.into_inner();

    warn!(
        deprecated_api = true,
        user_id = token.user_id,
        area_id,
        tag_name = args.name,
        tag_value = args.value,
        "User attempted to update area tag",
    );

    let area: Option<Area> = Area::select_by_id(&area_id, &conn)?;

    match area {
        Some(area) => {
            if args.value.len() > 0 {
                conn.execute(
                    area::INSERT_TAG,
                    named_params! {
                        ":area_id": area.id,
                        ":tag_name": format!("$.{}", args.name),
                        ":tag_value": args.value,
                    },
                )?;
            } else {
                conn.execute(
                    area::DELETE_TAG,
                    named_params! {
                        ":area_id": area.id,
                        ":tag_name": format!("$.{}", args.name),
                    },
                )?;
            }

            Ok(HttpResponse::Created())
        }
        None => Err(ApiError::new(
            404,
            &format!("There is no area with id {area_id}"),
        )),
    }
}

#[delete("{id}")]
async fn delete_by_id(
    id: Path<String>,
    req: HttpRequest,
    conn: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&conn, &req)?;
    let id = id.into_inner();

    warn!(
        user_id = token.user_id,
        area_id = id,
        "User attempted to delete an area",
    );

    let area: Option<Area> = Area::select_by_id(&id, &conn)?;

    match area {
        Some(_area) => {
            conn.execute(area::MARK_AS_DELETED, named_params! { ":id": id })?;

            Ok(HttpResponse::Ok())
        }
        None => Err(ApiError::new(
            404,
            &format!("There is no area with id {id}"),
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

        let area = Area::select_by_id("test-area", &Connection::open(db_path)?)?.unwrap();

        assert!(area.tags["string"].is_string());
        assert!(area.tags["int"].is_u64());
        assert!(area.tags["float"].is_f64());
        assert!(area.tags["bool"].is_boolean());

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

        conn.execute(area::INSERT, named_params! { ":id": "test" })?;

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

        conn.execute(
            "INSERT INTO area (id, updated_at) VALUES ('test1', '2023-05-05')",
            [],
        )?;
        conn.execute(
            "INSERT INTO area (id, updated_at) VALUES ('test2', '2023-05-06')",
            [],
        )?;
        conn.execute(
            "INSERT INTO area (id, updated_at) VALUES ('test3', '2023-05-07')",
            [],
        )?;

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
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        let area_id = "test";
        conn.execute(area::INSERT, named_params! { ":id": area_id })?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::get_by_id),
        )
        .await;

        let req = TestRequest::get().uri(&format!("/{area_id}")).to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, area_id);

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

        let area_id = "test";
        conn.execute(area::INSERT, named_params![":id": area_id])?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::patch_tags),
        )
        .await;
        let req = TestRequest::patch()
            .uri(&format!("/{area_id}/tags"))
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

        let area_id = "test";
        conn.execute(area::INSERT, named_params![":id": area_id])?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::patch_by_id),
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
            .uri(&format!("/{area_id}"))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_json(args)
            .to_request();

        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);

        let area = Area::select_by_id(&area_id, &Connection::open(db_path)?)?.unwrap();

        assert!(area.tags["string"].is_string());
        assert!(area.tags["unsigned"].is_u64());
        assert!(area.tags["float"].is_f64());
        assert!(area.tags["bool"].is_boolean());

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

        let area_id = "test";
        conn.execute(area::INSERT, named_params![":id": area_id])?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::post_tags),
        )
        .await;

        let req = TestRequest::post()
            .uri(&format!("/{area_id}/tags"))
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

        let area_id = "test";
        conn.execute(area::INSERT, named_params! { ":id": area_id })?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::delete_by_id),
        )
        .await;
        let req = TestRequest::delete()
            .uri(&format!("/{area_id}"))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);

        let area: Option<Area> = Area::select_by_id(&area_id, &Connection::open(db_path)?)?;

        assert!(area.is_some());

        assert!(area.unwrap().deleted_at.len() > 0);

        Ok(())
    }
}
