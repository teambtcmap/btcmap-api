use std::collections::HashMap;

use crate::model::area;
use crate::model::Area;
use crate::service::auth::is_from_admin;
use crate::ApiError;
use actix_web::get;
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
use serde_json::Value;

#[derive(Serialize, Deserialize)]
struct PostArgs {
    id: String,
}

#[derive(Deserialize)]
pub struct GetArgs {
    updated_since: Option<String>,
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
    is_from_admin(&req)?;

    if let Some(_) = db
        .query_row(
            area::SELECT_BY_ID,
            &[(":id", &args.id)],
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

#[get("")]
async fn get(args: Query<GetArgs>, db: Data<Connection>) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => db
            .prepare(area::SELECT_UPDATED_SINCE)?
            .query_map(
                named_params! { ":updated_since": updated_since },
                area::SELECT_UPDATED_SINCE_MAPPER,
            )?
            .map(|it| it.map(|it| it.into()))
            .collect::<Result<_, _>>()?,
        None => db
            .prepare(area::SELECT_ALL)?
            .query_map([], area::SELECT_ALL_MAPPER)?
            .map(|it| it.map(|it| it.into()))
            .collect::<Result<_, _>>()?,
    }))
}

#[get("{id}")]
async fn get_by_id(id: Path<String>, db: Data<Connection>) -> Result<Json<GetItem>, ApiError> {
    let id = id.into_inner();

    db.query_row(
        area::SELECT_BY_ID,
        &[(":id", &id)],
        area::SELECT_BY_ID_MAPPER,
    )
    .optional()?
    .map(|it| Json(it.into()))
    .ok_or(ApiError::new(
        404,
        &format!("Area with id {id} doesn't exist"),
    ))
}

#[post("{id}/tags")]
async fn post_tags(
    id: Path<String>,
    req: HttpRequest,
    args: Form<PostTagsArgs>,
    db: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    is_from_admin(&req)?;

    let id = id.into_inner();

    let area: Option<Area> = db
        .query_row(
            area::SELECT_BY_ID,
            &[(":id", &id)],
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
            &format!("There is no area with id {id}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::db::tests::db;
    use crate::Result;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};
    use std::env;

    #[actix_web::test]
    async fn post() -> Result<()> {
        let admin_token = "test";
        env::set_var("ADMIN_TOKEN", admin_token);
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db()?))
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
    async fn post_tags() -> Result<()> {
        let admin_token = "test";
        env::set_var("ADMIN_TOKEN", admin_token);
        let db = db()?;
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
}
