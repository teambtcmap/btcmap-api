use crate::model::admin_action;
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
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use serde_json::Map;
use serde_json::Value;

#[derive(Serialize, Deserialize)]
struct PostArgs {
    id: String,
}

#[derive(Serialize, Deserialize)]
struct PostJsonArgs {
    id: String,
    tags: Map<String, Value>,
}

#[derive(Deserialize)]
pub struct GetArgs {
    updated_since: Option<String>,
    limit: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct GetItem {
    pub id: String,
    pub tags: Map<String, Value>,
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
async fn post(
    args: Form<PostArgs>,
    req: HttpRequest,
    db: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&db, &req)?;

    db.execute(
        admin_action::INSERT,
        named_params! {
            ":user_id": token.user_id,
            ":message": format!("[legacy_api] User {} attempted to create area {}", token.user_id, args.id),
        },
    )?;

    if let Some(_) = db
        .query_row(
            area::SELECT_BY_ID,
            named_params! { ":id": args.id },
            area::SELECT_BY_ID_MAPPER,
        )
        .optional()?
    {
        Err(ApiError::new(
            303,
            format!("Area {} already exists", args.id),
        ))?
    }

    db.execute(area::INSERT, named_params![ ":id": args.id ])?;

    Ok(Json(json!({
        "message": format!("Area {} has been created", args.id),
    })))
}

#[post("")]
async fn post_json(
    args: Json<PostJsonArgs>,
    req: HttpRequest,
    db: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&db, &req)?;

    db.execute(
        admin_action::INSERT,
        named_params! {
            ":user_id": token.user_id,
            ":message": format!("User {} attempted to create area {}", token.user_id, args.id),
        },
    )?;

    if let Some(_) = db
        .query_row(
            area::SELECT_BY_ID,
            named_params! { ":id": args.id },
            area::SELECT_BY_ID_MAPPER,
        )
        .optional()?
    {
        Err(ApiError::new(
            303,
            format!("Area {} already exists", args.id),
        ))?
    }

    db.execute(area::INSERT, named_params![ ":id": args.id ])?;

    db.execute(
        area::UPDATE_TAGS,
        named_params![ ":area_id": args.id, ":tags": serde_json::to_string(&args.tags).unwrap() ],
    )?;

    Ok(Json(json!({
        "message": format!("Area {} has been created", args.id),
    })))
}

#[get("")]
async fn get(args: Query<GetArgs>, db: Data<Connection>) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => db
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
        None => db
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
async fn get_by_id(id: Path<String>, db: Data<Connection>) -> Result<Json<GetItem>, ApiError> {
    let id = id.into_inner();

    db.query_row(
        area::SELECT_BY_ID,
        named_params! { ":id": id },
        area::SELECT_BY_ID_MAPPER,
    )
    .optional()?
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
    db: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&db, &req)?;
    let area_id = id.into_inner();

    db.execute(
        admin_action::INSERT,
        named_params! {
            ":user_id": token.user_id,
            ":message": format!("User {} attempted to update tags for area {}", token.user_id, area_id),
        },
    )?;

    let area: Option<Area> = db
        .query_row(
            area::SELECT_BY_ID,
            named_params! { ":id": area_id },
            area::SELECT_BY_ID_MAPPER,
        )
        .optional()?;

    let area = match area {
        Some(v) => v,
        None => {
            return Err(ApiError::new(
                404,
                &format!("There is no area with id {area_id}"),
            ));
        }
    };

    let mut new_tags = match args.tags.clone() {
        Value::Object(v) => v,
        _ => {
            return Err(ApiError::new(
                400,
                &format!("The field tags should be an object"),
            ));
        }
    };

    let mut old_tags = area.tags.clone();

    let mut merged_tags = Map::new();
    merged_tags.append(&mut old_tags);
    merged_tags.append(&mut new_tags);

    db.execute(
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
    args: Json<Map<String, Value>>,
    db: Data<Connection>,
    id: Path<String>,
    req: HttpRequest,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&db, &req)?;
    let area_id = id.into_inner();

    let keys: Vec<String> = args.keys().map(|it| it.to_string()).collect();

    db.execute(
        admin_action::INSERT,
        named_params! {
            ":user_id": token.user_id,
            ":message": format!(
                "User {} attempted to update tags {} for area {}",
                token.user_id,
                keys.join(", "),
                area_id,
            ),
        },
    )?;

    let area: Option<Area> = db
        .query_row(
            area::SELECT_BY_ID,
            named_params! { ":id": area_id },
            area::SELECT_BY_ID_MAPPER,
        )
        .optional()?;

    let area = match area {
        Some(v) => v,
        None => {
            return Err(ApiError::new(
                404,
                &format!("There is no area with id {area_id}"),
            ));
        }
    };

    let mut old_tags = area.tags.clone();

    let mut merged_tags = Map::new();
    merged_tags.append(&mut old_tags);
    merged_tags.append(&mut args.clone());

    db.execute(
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
    db: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&db, &req)?;
    let area_id = id.into_inner();

    db.execute(
        admin_action::INSERT,
        named_params! {
            ":user_id": token.user_id,
            ":message": format!("[deprecated_api] User {} attempted to update tag {} for area {}", token.user_id, args.name, area_id),
        },
    )?;

    let area: Option<Area> = db
        .query_row(
            area::SELECT_BY_ID,
            named_params! { ":id": area_id },
            area::SELECT_BY_ID_MAPPER,
        )
        .optional()?;

    match area {
        Some(area) => {
            if args.value.len() > 0 {
                db.execute(
                    area::INSERT_TAG,
                    named_params! {
                        ":area_id": area.id,
                        ":tag_name": format!("$.{}", args.name),
                        ":tag_value": args.value,
                    },
                )?;
            } else {
                db.execute(
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
    db: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&db, &req)?;
    let id = id.into_inner();

    db.execute(
        admin_action::INSERT,
        named_params! {
            ":user_id": token.user_id,
            ":message": format!("User {} attempted to delete area {}", token.user_id, id),
        },
    )?;

    let area: Option<Area> = db
        .query_row(
            area::SELECT_BY_ID,
            &[(":id", &id)],
            area::SELECT_BY_ID_MAPPER,
        )
        .optional()?;

    match area {
        Some(_area) => {
            db.execute(area::MARK_AS_DELETED, named_params! { ":id": id })?;

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
    use crate::command::db::tests::db;
    use crate::model::token;
    use crate::Result;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn post() -> Result<()> {
        let admin_token = "test";
        let db = db()?;
        db.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db))
                .service(scope("/").service(super::post)),
        )
        .await;
        let req = TestRequest::post()
            .uri("/")
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_form(PostArgs {
                id: "test-area".into(),
            })
            .to_request();
        let res = test::call_service(&app, req).await;
        log::info!("Response status: {}", res.status());
        assert!(res.status().is_success());
        Ok(())
    }

    #[actix_web::test]
    async fn post_json() -> Result<()> {
        let admin_token = "test";
        let db = db()?;
        db.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        let db_clone = Connection::open(db.path().unwrap())?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db))
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
        log::info!("Response status: {}", res.status());
        assert!(res.status().is_success());

        let area = db_clone.query_row(
            area::SELECT_BY_ID,
            &[(":id", "test-area")],
            area::SELECT_BY_ID_MAPPER,
        )?;

        assert!(area.tags["string"].is_string());
        assert!(area.tags["int"].is_u64());
        assert!(area.tags["float"].is_f64());
        assert!(area.tags["bool"].is_boolean());

        Ok(())
    }

    #[actix_web::test]
    async fn get_empty_table() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db()?))
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
        let db = db()?;
        db.execute(area::INSERT, named_params! { ":id": "test" })?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db))
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
        let db = db()?;
        db.execute(area::INSERT, named_params! { ":id": "test1" })?;
        db.execute(area::INSERT, named_params! { ":id": "test2" })?;
        db.execute(area::INSERT, named_params! { ":id": "test3" })?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db))
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
        let db = db()?;
        let area_id = "test";
        db.execute(area::INSERT, named_params! { ":id": area_id })?;
        let app =
            test::init_service(App::new().app_data(Data::new(db)).service(super::get_by_id)).await;
        let req = TestRequest::get().uri(&format!("/{area_id}")).to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, area_id);
        Ok(())
    }

    #[actix_web::test]
    async fn patch_tags() -> Result<()> {
        let admin_token = "test";
        let db = db()?;
        db.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        let area_id = "test";
        db.execute(area::INSERT, named_params![":id": area_id])?;
        let app =
            test::init_service(App::new().app_data(Data::new(db)).service(super::patch_tags)).await;
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
        let admin_token = "test";
        let db = db()?;
        db.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        let db_clone = Connection::open(db.path().unwrap())?;
        let area_id = "test";
        db.execute(area::INSERT, named_params![":id": area_id])?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db))
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

        let area = db_clone.query_row(
            area::SELECT_BY_ID,
            &[(":id", &area_id)],
            area::SELECT_BY_ID_MAPPER,
        )?;

        assert!(area.tags["string"].is_string());
        assert!(area.tags["unsigned"].is_u64());
        assert!(area.tags["float"].is_f64());
        assert!(area.tags["bool"].is_boolean());

        Ok(())
    }

    #[actix_web::test]
    async fn post_tags() -> Result<()> {
        let admin_token = "test";
        let db = db()?;
        db.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        let area_id = "test";
        db.execute(area::INSERT, named_params![":id": area_id])?;
        let app =
            test::init_service(App::new().app_data(Data::new(db)).service(super::post_tags)).await;
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
        let admin_token = "test";
        let db = db()?;
        db.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;
        let db_clone = Connection::open(db.path().unwrap())?;
        let area_id = "test";
        db.execute(area::INSERT, named_params! { ":id": area_id })?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db))
                .service(super::delete_by_id),
        )
        .await;
        let req = TestRequest::delete()
            .uri(&format!("/{area_id}"))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);

        let area: Option<Area> = db_clone
            .query_row(
                area::SELECT_BY_ID,
                &[(":id", &area_id)],
                area::SELECT_BY_ID_MAPPER,
            )
            .optional()?;

        assert!(area.is_some());

        assert!(area.unwrap().deleted_at.len() > 0);

        Ok(())
    }
}
