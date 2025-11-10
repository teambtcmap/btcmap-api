use crate::db;
use crate::db::event::schema::Event;
use crate::log::RequestExtension;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::HttpMessage;
use actix_web::HttpRequest;
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Item {
    pub id: i64,
    pub lat: f64,
    pub lon: f64,
    pub name: String,
    pub website: String,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
}

impl From<Event> for Item {
    fn from(val: Event) -> Self {
        Item {
            id: val.id,
            lat: val.lat,
            lon: val.lon,
            name: val.name,
            website: val.website,
            starts_at: val.starts_at.unwrap_or(OffsetDateTime::UNIX_EPOCH),
            ends_at: val.ends_at,
        }
    }
}

#[get("")]
pub async fn get(req: HttpRequest, pool: Data<Pool>) -> RestResult<Vec<Item>> {
    let items = db::event::queries::select_all(&pool)
        .await
        .map_err(|_| RestApiError::database())?;
    let items: Vec<Event> = items
        .into_iter()
        .filter(|it| {
            it.deleted_at.is_none()
                && (it.starts_at.is_none() || it.starts_at > Some(OffsetDateTime::now_utc()))
        })
        .collect();
    req.extensions_mut()
        .insert(RequestExtension::new(items.len()));
    Ok(Json(items.into_iter().map(|it| it.into()).collect()))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Pool>) -> RestResult<Item> {
    db::event::queries::select_by_id(id.into_inner(), &pool)
        .await
        .map(|it| Json(it.into()))
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })
}
