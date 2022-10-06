use crate::db;
use crate::model::ApiError;
use crate::model::User;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use rusqlite::Connection;
use std::sync::Mutex;

#[get("/users")]
async fn get(conn: Data<Mutex<Connection>>) -> Result<Json<Vec<User>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .prepare(db::USER_SELECT_ALL)?
            .query_map([], db::mapper_user_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap())
            .collect(),
    ))
}
