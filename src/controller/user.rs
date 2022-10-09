use crate::db;
use crate::model::ApiError;
use crate::model::User;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use serde::Serialize;
use serde_json::Value;
use std::sync::Mutex;

#[derive(Serialize)]
pub struct GetUserItem {
    pub id: String,
    pub data: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl Into<GetUserItem> for User {
    fn into(self) -> GetUserItem {
        GetUserItem {
            id: self.id,
            data: self.data,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

#[derive(Serialize)]
pub struct GetUserItemV2 {
    pub id: String,
    pub osm_json: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl Into<GetUserItemV2> for User {
    fn into(self) -> GetUserItemV2 {
        GetUserItemV2 {
            id: self.id,
            osm_json: self.data,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

#[get("/users")]
async fn get(conn: Data<Mutex<Connection>>) -> Result<Json<Vec<GetUserItem>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .prepare(db::USER_SELECT_ALL)?
            .query_map([], db::mapper_user_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
    ))
}

#[get("/v2/users")]
async fn get_v2(conn: Data<Mutex<Connection>>) -> Result<Json<Vec<GetUserItemV2>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .prepare(db::USER_SELECT_ALL)?
            .query_map([], db::mapper_user_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
    ))
}

#[get("/users/{id}")]
pub async fn get_by_id(
    path: Path<String>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Option<GetUserItem>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .query_row(
                db::USER_SELECT_BY_ID,
                [path.into_inner()],
                db::mapper_user_full(),
            )
            .optional()?
            .map(|it| it.into()),
    ))
}

#[get("/v2/users/{id}")]
pub async fn get_by_id_v2(
    path: Path<String>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Option<GetUserItemV2>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .query_row(
                db::USER_SELECT_BY_ID,
                [path.into_inner()],
                db::mapper_user_full(),
            )
            .optional()?
            .map(|it| it.into()),
    ))
}
