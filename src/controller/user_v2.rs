use crate::auth::is_from_admin;
use crate::model::user;
use crate::model::ApiError;
use crate::model::User;
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

#[derive(Deserialize)]
pub struct GetArgs {
    updated_since: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GetItem {
    pub id: i64,
    pub osm_json: Value,
    pub tags: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

impl Into<GetItem> for User {
    fn into(self) -> GetItem {
        GetItem {
            id: self.id,
            osm_json: self.osm_json,
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

#[get("")]
async fn get(
    args: Query<GetArgs>,
    db: Data<Mutex<Connection>>,
) -> Result<Json<Vec<GetItem>>, ApiError> {
    let db = db.lock()?;

    Ok(Json(match &args.updated_since {
        Some(updated_since) => db
            .prepare(user::SELECT_UPDATED_SINCE)?
            .query_map(
                &[(":updated_since", updated_since)],
                user::SELECT_UPDATED_SINCE_MAPPER,
            )?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
        None => db
            .prepare(user::SELECT_ALL)?
            .query_map([], user::SELECT_ALL_MAPPER)?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
    }))
}

#[get("{id}")]
pub async fn get_by_id(
    id: Path<String>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<GetItem>, ApiError> {
    let id = id.into_inner();

    conn.lock()?
        .query_row(
            user::SELECT_BY_ID,
            &[(":id", &id)],
            user::SELECT_BY_ID_MAPPER,
        )
        .optional()?
        .map(|it| Json(it.into()))
        .ok_or(ApiError::new(
            404,
            &format!("User with id {id} doesn't exist"),
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

    let user: Option<User> = conn
        .query_row(
            user::SELECT_BY_ID,
            &[(":id", &id)],
            user::SELECT_BY_ID_MAPPER,
        )
        .optional()?;

    match user {
        Some(user) => {
            if args.value.len() > 0 {
                conn.execute(
                    user::INSERT_TAG,
                    named_params! {
                        ":user_id": &user.id,
                        ":tag_name": format!("$.{}", &args.name),
                        ":tag_value": args.value,
                    },
                )?;
            } else {
                conn.execute(
                    user::DELETE_TAG,
                    named_params! {
                        ":user_id": &user.id,
                        ":tag_name": format!("$.{}", &args.name),
                    },
                )?;
            }

            Ok(HttpResponse::Created())
        }
        None => Err(ApiError::new(
            404,
            &format!("There is no user with id {id}"),
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
    use rusqlite::named_params;
    use serde_json::Value;
    use std::sync::atomic::Ordering;

    #[actix_web::test]
    async fn get_empty_table() {
        let db_name = db::COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut db =
            Connection::open(format!("file::testdb_{db_name}:?mode=memory&cache=shared")).unwrap();
        db::migrate(&mut db).unwrap();
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
        db.execute(
            user::INSERT,
            named_params! {
                ":id": 1,
                ":osm_json": "{}",
            },
        )
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
    async fn get_updated_since() {
        let db_name = db::COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut db =
            Connection::open(format!("file::testdb_{db_name}:?mode=memory&cache=shared")).unwrap();
        db::migrate(&mut db).unwrap();
        db.execute(
            "INSERT INTO user (id, osm_json, updated_at) VALUES (1, '{}', '2022-01-05')",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO user (id, osm_json, updated_at) VALUES (2, '{}', '2022-02-05')",
            [],
        )
        .unwrap();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(Mutex::new(db)))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2022-01-10")
            .to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 1);
    }

    #[actix_web::test]
    async fn get_by_id() {
        let db_name = db::COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut db =
            Connection::open(format!("file::testdb_{db_name}:?mode=memory&cache=shared")).unwrap();
        db::migrate(&mut db).unwrap();
        let element_id = 1;
        db.execute(
            user::INSERT,
            named_params! {
                ":id": &element_id,
                ":osm_json": "{}",
            },
        )
        .unwrap();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(Mutex::new(db)))
                .service(super::get_by_id),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{element_id}"))
            .to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, element_id);
    }
}
