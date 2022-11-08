use crate::model::report;
use crate::model::ApiError;
use crate::model::Report;
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
pub struct GetArgs {
    updated_since: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GetItem {
    pub id: i64,
    pub area_id: String,
    pub date: String,
    pub tags: Value,
    pub total_elements: i64,
    pub total_elements_onchain: i64,
    pub total_elements_lightning: i64,
    pub total_elements_lightning_contactless: i64,
    pub up_to_date_elements: i64,
    pub outdated_elements: i64,
    pub legacy_elements: i64,
    pub elements_created: i64,
    pub elements_updated: i64,
    pub elements_deleted: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

impl Into<GetItem> for Report {
    fn into(self) -> GetItem {
        GetItem {
            id: self.id,
            area_id: self.area_id,
            date: self.date,
            tags: self.tags.clone(),
            total_elements: self.tags["total_elements"].as_i64().unwrap_or(0),
            total_elements_onchain: self.tags["total_elements_onchain"].as_i64().unwrap_or(0),
            total_elements_lightning: self.tags["total_elements_lightning"].as_i64().unwrap_or(0),
            total_elements_lightning_contactless: self.tags["total_elements_lightning_contactless"]
                .as_i64()
                .unwrap_or(0),
            up_to_date_elements: self.tags["up_to_date_elements"].as_i64().unwrap_or(0),
            outdated_elements: self.tags["outdated_elements"].as_i64().unwrap_or(0),
            legacy_elements: self.tags["legacy_elements"].as_i64().unwrap_or(0),
            elements_created: 0,
            elements_updated: 0,
            elements_deleted: 0,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
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
            .prepare(report::SELECT_UPDATED_SINCE)?
            .query_map(
                &[(":updated_since", updated_since)],
                report::SELECT_UPDATED_SINCE_MAPPER,
            )?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
        None => conn
            .lock()?
            .prepare(report::SELECT_ALL)?
            .query_map([], report::SELECT_ALL_MAPPER)?
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
            report::SELECT_BY_ID,
            &[(":id", &id)],
            report::SELECT_BY_ID_MAPPER,
        )
        .optional()?
        .map(|it| Json(it.into()))
        .ok_or(ApiError::new(
            404,
            &format!("Report with id {id} doesn't exist"),
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
            report::INSERT,
            named_params! {
                ":area_id" : "",
                ":date" : "",
                ":tags" : "{}",
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
            "INSERT INTO report (area_id, date, updated_at) VALUES ('', '', '2022-01-05')",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO report (area_id, date, updated_at) VALUES ('', '', '2022-02-05')",
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
}
