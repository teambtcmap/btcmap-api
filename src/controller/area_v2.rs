use crate::auth::is_from_admin;
use crate::model::area;
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

#[derive(Serialize, Deserialize)]
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

#[post("")]
async fn post(
    args: Form<PostArgs>,
    req: HttpRequest,
    conn: Data<Mutex<Connection>>,
) -> Result<impl Responder, ApiError> {
    if let Err(err) = is_from_admin(&req) {
        return Err(err);
    };

    conn.lock()?
        .execute(area::INSERT, named_params![ ":id": args.id ])?;

    Ok(HttpResponse::Created())
}

#[get("")]
async fn get(
    args: Query<GetArgs>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => conn
            .lock()?
            .prepare(area::SELECT_UPDATED_SINCE)?
            .query_map(
                &[(":updated_since", &updated_since)],
                area::SELECT_UPDATED_SINCE_MAPPER,
            )?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
        None => conn
            .lock()?
            .prepare(area::SELECT_ALL)?
            .query_map([], area::SELECT_ALL_MAPPER)?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
    }))
}

#[get("{id}")]
async fn get_by_id(
    id: Path<String>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<GetItem>, ApiError> {
    let id = id.into_inner();

    conn.lock()?
        .query_row(
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
    conn: Data<Mutex<Connection>>,
) -> Result<impl Responder, ApiError> {
    if let Err(err) = is_from_admin(&req) {
        return Err(err);
    };

    let id = id.into_inner();
    let conn = conn.lock()?;

    let area: Option<Area> = conn
        .query_row(
            area::SELECT_BY_ID,
            &[(":id", &id)],
            area::SELECT_BY_ID_MAPPER,
        )
        .optional()?;

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
            &format!("There is no area with id {id}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
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
    }

    #[actix_web::test]
    async fn get_empty_table() {
        let db_name = db::COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut db =
            Connection::open(format!("file::testdb_{db_name}:?mode=memory&cache=shared")).unwrap();
        db::migrate(&mut db).unwrap();
        db.execute("DELETE FROM area", []).unwrap();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(Mutex::new(db)))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 0);
    }

    #[actix_web::test]
    async fn get_one_row() {
        let db_name = db::COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut db =
            Connection::open(format!("file::testdb_{db_name}:?mode=memory&cache=shared")).unwrap();
        db::migrate(&mut db).unwrap();
        db.execute("DELETE FROM area", []).unwrap();
        db.execute(area::INSERT, named_params! { ":id": "test" })
            .unwrap();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(Mutex::new(db)))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 1);
    }

    #[actix_web::test]
    async fn get_by_id() {
        let db_name = db::COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut db =
            Connection::open(format!("file::testdb_{db_name}:?mode=memory&cache=shared")).unwrap();
        db::migrate(&mut db).unwrap();
        db.execute("DELETE FROM area", []).unwrap();
        let area_id = "test";
        db.execute(area::INSERT, named_params! { ":id": area_id })
            .unwrap();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(Mutex::new(db)))
                .service(super::get_by_id),
        )
        .await;
        let req = TestRequest::get().uri(&format!("/{area_id}")).to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, area_id);
    }
}
