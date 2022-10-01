use crate::db;
use crate::model::ApiError;
use crate::model::Element;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use serde::Deserialize;
use std::sync::Mutex;

#[derive(Deserialize)]
pub struct GetElementsArgs {
    updated_since: Option<String>,
}

#[actix_web::get("/elements")]
pub async fn get_elements(
    args: Query<GetElementsArgs>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Vec<Element>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => conn
            .lock()?
            .prepare(db::ELEMENT_SELECT_UPDATED_SINCE)?
            .query_map([updated_since], db::mapper_element_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap())
            .collect(),
        None => conn
            .lock()?
            .prepare(db::ELEMENT_SELECT_ALL)?
            .query_map([], db::mapper_element_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap())
            .collect(),
    }))
}

#[actix_web::get("/elements/{id}")]
pub async fn get_element(
    path: Path<String>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Option<Element>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .query_row(
                db::ELEMENT_SELECT_BY_ID,
                [path.into_inner()],
                db::mapper_element_full(),
            )
            .optional()?,
    ))
}
