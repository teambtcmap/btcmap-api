use crate::db;
use crate::model::ApiError;
use crate::model::Element;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::sync::Mutex;

#[derive(Deserialize)]
pub struct GetElementsArgs {
    updated_since: Option<String>,
}

#[derive(Serialize)]
pub struct GetElementItem {
    pub id: String,
    pub data: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl Into<GetElementItem> for Element {
    fn into(self) -> GetElementItem {
        GetElementItem {
            id: self.id,
            data: self.data,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

#[derive(Serialize)]
pub struct GetElementItemV2 {
    pub id: String,
    pub osm_json: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

impl Into<GetElementItemV2> for Element {
    fn into(self) -> GetElementItemV2 {
        GetElementItemV2 {
            id: self.id,
            osm_json: self.data,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at.unwrap_or("".to_string()),
        }
    }
}

#[get("/elements")]
pub async fn get(
    args: Query<GetElementsArgs>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Vec<GetElementItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => conn
            .lock()?
            .prepare(db::ELEMENT_SELECT_UPDATED_SINCE)?
            .query_map([updated_since], db::mapper_element_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
        None => conn
            .lock()?
            .prepare(db::ELEMENT_SELECT_ALL)?
            .query_map([], db::mapper_element_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
    }))
}

#[get("/v2/elements")]
pub async fn get_v2(
    args: Query<GetElementsArgs>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Vec<GetElementItemV2>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => conn
            .lock()?
            .prepare(db::ELEMENT_SELECT_UPDATED_SINCE)?
            .query_map([updated_since], db::mapper_element_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
        None => conn
            .lock()?
            .prepare(db::ELEMENT_SELECT_ALL)?
            .query_map([], db::mapper_element_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
    }))
}

#[get("/elements/{id}")]
pub async fn get_by_id(
    path: Path<String>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Option<GetElementItem>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .query_row(
                db::ELEMENT_SELECT_BY_ID,
                [path.into_inner()],
                db::mapper_element_full(),
            )
            .optional()?
            .map(|it| it.into()),
    ))
}

#[get("/v2/elements/{id}")]
pub async fn get_by_id_v2(
    path: Path<String>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Option<GetElementItemV2>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .query_row(
                db::ELEMENT_SELECT_BY_ID,
                [path.into_inner()],
                db::mapper_element_full(),
            )
            .optional()?
            .map(|it| it.into()),
    ))
}
