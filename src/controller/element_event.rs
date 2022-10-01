use crate::model::ElementEvent;
use crate::model::ApiError;
use crate::db;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use rusqlite::Connection;
use std::sync::Mutex;

#[get("/element_events")]
async fn get(
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Vec<ElementEvent>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .prepare(db::ELEMENT_EVENT_SELECT_ALL)?
            .query_map([], db::mapper_element_event_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap())
            .collect(),
    ))
}
