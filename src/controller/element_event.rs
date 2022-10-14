use rusqlite::OptionalExtension;
use actix_web::web::Path;
use crate::db;
use crate::model::ApiError;
use crate::model::ElementEvent;
use crate::model::User;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Query;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Mutex;

#[derive(Serialize)]
pub struct GetElementEventsItem {
    pub date: String,
    pub element_id: String,
    pub element_lat: f64,
    pub element_lon: f64,
    pub element_name: String,
    pub event_type: String,
    pub user_id: i64,
    pub user: Option<String>,
    pub user_v2: Option<User>,
}

#[derive(Deserialize)]
pub struct GetEventItemsArgsV2 {
    updated_since: Option<String>,
}

#[derive(Serialize)]
pub struct GetEventItemV2 {
    pub id: i64,
    pub date: String,
    pub r#type: String,
    pub element_id: String,
    pub element_lat: f64,
    pub element_lon: f64,
    pub user_id: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

impl Into<GetEventItemV2> for ElementEvent {
    fn into(self) -> GetEventItemV2 {
        GetEventItemV2 {
            id: self.id,
            date: self.date,
            r#type: self.event_type,
            element_id: self.element_id,
            element_lat: self.element_lat,
            element_lon: self.element_lon,
            user_id: self.user_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

#[get("/element_events")]
async fn get(conn: Data<Mutex<Connection>>) -> Result<Json<Vec<GetElementEventsItem>>, ApiError> {
    let conn = conn.lock()?;

    let element_events: Vec<ElementEvent> = conn
        .prepare(db::ELEMENT_EVENT_SELECT_ALL)?
        .query_map([], db::mapper_element_event_full())?
        .filter(|it| it.is_ok())
        .map(|it| it.unwrap())
        .collect();

    let users: Vec<User> = conn
        .prepare(db::USER_SELECT_ALL)?
        .query_map([], db::mapper_user_full())?
        .filter(|it| it.is_ok())
        .map(|it| it.unwrap())
        .collect();

    let res: Vec<GetElementEventsItem> = element_events
        .iter()
        .map(|event| {
            let user = users
                .iter()
                .find(|it| it.id == event.user_id.to_string())
                .map(|it| User {
                    id: it.id.clone(),
                    data: it.data.clone(),
                    created_at: it.created_at.clone(),
                    updated_at: it.updated_at.clone(),
                    deleted_at: it.deleted_at.clone(),
                });

            GetElementEventsItem {
                date: event.date.clone(),
                element_id: event.element_id.clone(),
                element_lat: event.element_lat,
                element_lon: event.element_lon,
                element_name: event.element_name.clone(),
                event_type: event.event_type.clone(),
                user_id: event.user_id,
                user: event.user.clone(),
                user_v2: user,
            }
        })
        .collect();

    Ok(Json(res))
}

#[get("/v2/events")]
async fn get_v2(
    args: Query<GetEventItemsArgsV2>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Vec<GetEventItemV2>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => conn
            .lock()?
            .prepare(db::ELEMENT_EVENT_SELECT_UPDATED_SINCE)?
            .query_map([updated_since], db::mapper_element_event_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
        None => conn
            .lock()?
            .prepare(db::ELEMENT_EVENT_SELECT_ALL)?
            .query_map([], db::mapper_element_event_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
    }))
}

#[get("/v2/events/{id}")]
pub async fn get_by_id_v2(
    path: Path<String>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Option<GetEventItemV2>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .query_row(
                db::ELEMENT_EVENT_SELECT_BY_ID,
                [path.into_inner()],
                db::mapper_element_event_full(),
            )
            .optional()?
            .map(|it| it.into()),
    ))
}