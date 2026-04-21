use crate::db;
use crate::db::main::event::schema::Event;
use crate::db::main::MainPool;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Item {
    pub id: i64,
    pub area_id: Option<i64>,
    pub lat: f64,
    pub lon: f64,
    pub name: String,
    pub website: String,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
    pub cron_schedule: Option<String>,
}

impl From<Event> for Item {
    fn from(val: Event) -> Self {
        Item {
            id: val.id,
            area_id: val.area_id,
            lat: val.lat,
            lon: val.lon,
            name: val.name,
            website: val.website,
            starts_at: val.starts_at.unwrap_or(OffsetDateTime::UNIX_EPOCH),
            ends_at: val.ends_at,
            cron_schedule: val.cron_schedule,
        }
    }
}

#[get("")]
pub async fn get(pool: Data<MainPool>) -> RestResult<Vec<Item>> {
    let items = db::main::event::queries::select_all(&pool)
        .await
        .map_err(|_| RestApiError::database())?;
    let items: Vec<Event> = items
        .into_iter()
        .filter(|it| {
            it.deleted_at.is_none()
                && (it.starts_at.is_none() || it.starts_at > Some(OffsetDateTime::now_utc()))
        })
        .collect();
    Ok(Json(items.into_iter().map(|it| it.into()).collect()))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<MainPool>) -> RestResult<Item> {
    db::main::event::queries::select_by_id(id.into_inner(), &pool)
        .await
        .map(|it| Json(it.into()))
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })
}

#[cfg(test)]
mod test {
    use crate::db::main::test::pool;
    use crate::{db, Result};
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use geojson::JsonObject;
    use time::macros::datetime;
    use time::OffsetDateTime;

    #[test]
    async fn get_empty_array() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_not_empty_array() -> Result<()> {
        let pool = pool();
        let event = db::main::event::queries::insert(
            Some(1),
            1.23,
            4.56,
            "name".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(event.id, res.first().unwrap()["id"].as_i64().unwrap());
        assert_eq!(1, res.first().unwrap()["area_id"].as_i64().unwrap());
        Ok(())
    }

    #[test]
    async fn get_excludes_deleted() -> Result<()> {
        let pool = pool();
        let event = db::main::event::queries::insert(
            None,
            1.23,
            4.56,
            "name".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            &pool,
        )
        .await?;
        db::main::event::queries::set_deleted_at(event.id, Some(OffsetDateTime::now_utc()), &pool)
            .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_excludes_past_events() -> Result<()> {
        let pool = pool();
        db::main::event::queries::insert(
            None,
            1.23,
            4.56,
            "past_event".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2020-01-01 0:00 UTC)),
            None,
            &pool,
        )
        .await?;
        let future_event = db::main::event::queries::insert(
            None,
            7.89,
            10.11,
            "future_event".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(
            future_event.id,
            res.first().unwrap()["id"].as_i64().unwrap()
        );
        Ok(())
    }

    #[test]
    async fn get_by_id() -> Result<()> {
        let pool = pool();
        let event = db::main::event::queries::insert(
            Some(1),
            1.23,
            4.56,
            "name".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_id),
        )
        .await;
        let req = TestRequest::get().uri("/1").to_request();
        let res: JsonObject = test::call_and_read_body_json(&app, req).await;
        assert_eq!(event.id, res["id"].as_i64().unwrap());
        assert_eq!(1, res["area_id"].as_i64().unwrap());
        Ok(())
    }

    #[test]
    async fn get_by_id_not_found() -> Result<()> {
        let pool = pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_id),
        )
        .await;
        let req = TestRequest::get().uri("/999").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 404);
        Ok(())
    }
}
