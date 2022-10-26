use crate::model::json::Json;
use crate::db;
use crate::model::ApiError;
use crate::model::User;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Path;
use actix_web::web::Query;
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

#[derive(Serialize)]
pub struct GetItem {
    pub id: String,
    pub osm_json: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

impl Into<GetItem> for User {
    fn into(self) -> GetItem {
        GetItem {
            id: self.id,
            osm_json: self.data,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at.unwrap_or("".to_string()),
        }
    }
}

#[get("")]
async fn get(
    args: Query<GetArgs>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => conn
            .lock()?
            .prepare(db::USER_SELECT_UPDATED_SINCE)?
            .query_map([updated_since], db::mapper_user_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
        None => conn
            .lock()?
            .prepare(db::USER_SELECT_ALL)?
            .query_map([], db::mapper_user_full())?
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
        .query_row(db::USER_SELECT_BY_ID, [&id], db::mapper_user_full())
        .optional()?
        .map(|it| Json(it.into()))
        .ok_or(ApiError::new(
            404,
            &format!("User with id {id} doesn't exist"),
        ))
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
            db::USER_INSERT,
            named_params! {
                ":id": 1,
                ":data": "{}",
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
}
