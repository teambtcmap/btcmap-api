use crate::auth::is_from_admin;
use crate::db;
use crate::model::ApiError;
use crate::model::Area;
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
use serde_json::Value;
use std::sync::Mutex;

#[derive(Serialize, Deserialize)]
struct PostArgs {
    id: String,
}

#[derive(Deserialize)]
pub struct GetArgs {
    updated_since: Option<String>,
}

#[derive(Serialize)]
pub struct GetItem {
    pub id: String,
    pub tags: Value,
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

#[derive(Deserialize)]
struct PostTagsArgs {
    name: String,
    value: String,
}

#[post("/v2/areas")]
async fn post(
    args: Form<PostArgs>,
    req: HttpRequest,
    conn: Data<Mutex<Connection>>,
) -> Result<impl Responder, ApiError> {
    if let Err(err) = is_from_admin(&req) {
        return Err(err);
    };

    conn.lock()?.execute(
        db::AREA_INSERT,
        named_params![
            ":id": args.id,
        ],
    )?;

    Ok(HttpResponse::Created())
}

#[get("/v2/areas")]
async fn get(
    args: Query<GetArgs>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => conn
            .lock()?
            .prepare(db::AREA_SELECT_UPDATED_SINCE)?
            .query_map([updated_since], db::mapper_area_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
        None => conn
            .lock()?
            .prepare(db::AREA_SELECT_ALL)?
            .query_map([], db::mapper_area_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
    }))
}

#[get("/v2/areas/{id}")]
async fn get_by_id(
    id: Path<String>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Option<GetItem>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .query_row(
                db::AREA_SELECT_BY_ID,
                [id.into_inner()],
                db::mapper_area_full(),
            )
            .optional()?
            .map(|it| it.into()),
    ))
}

#[post("/v2/areas/{id}/tags")]
async fn post_tags(
    id: Path<String>,
    req: HttpRequest,
    args: Form<PostTagsArgs>,
    conn: Data<Mutex<Connection>>,
) -> Result<impl Responder, ApiError> {
    if let Err(err) = is_from_admin(&req) {
        return Err(err);
    };

    let id = id.into_inner();
    let conn = conn.lock()?;

    let area: Option<Area> = conn
        .query_row(db::AREA_SELECT_BY_ID, [&id], db::mapper_area_full())
        .optional()?;

    match area {
        Some(area) => {
            if args.value.len() > 0 {
                conn.execute(
                    db::AREA_INSERT_TAG,
                    named_params! {
                        ":area_id": area.id,
                        ":tag_name": format!("$.{}", args.name),
                        ":tag_value": args.value,
                    },
                )?;
            } else {
                conn.execute(
                    db::AREA_DELETE_TAG,
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
    use crate::db;
    use actix_web::test::TestRequest;
    use actix_web::{test, App};
    use std::env;
    use std::sync::atomic::Ordering;

    #[actix_web::test]
    async fn post() {
        let admin_token = "test";
        env::set_var("ADMIN_TOKEN", admin_token);
        let db_name = db::COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut db =
            Connection::open(format!("file::testdb_{db_name}:?mode=memory&cache=shared")).unwrap();
        db::migrate(&mut db).unwrap();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(Mutex::new(db)))
                .service(super::post),
        )
        .await;
        let req = TestRequest::post()
            .uri("/v2/areas")
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_form(PostArgs {
                id: "test-area".into(),
            })
            .to_request();
        let res = test::call_service(&app, req).await;
        log::info!("Response status: {}", res.status());
        assert!(res.status().is_success());
    }
}
