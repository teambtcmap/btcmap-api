use crate::db;
use crate::model::ApiError;
use crate::model::DailyReport;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use serde::Serialize;
use std::sync::Mutex;

#[derive(Serialize)]
pub struct GetItem {
    pub id: i64,
    pub area_id: String,
    pub date: String,
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

impl Into<GetItem> for DailyReport {
    fn into(self) -> GetItem {
        GetItem {
            id: self.id,
            area_id: self.area_id,
            date: self.date,
            total_elements: self.total_elements,
            total_elements_onchain: self.total_elements_onchain,
            total_elements_lightning: self.total_elements_lightning,
            total_elements_lightning_contactless: self.total_elements_lightning_contactless,
            up_to_date_elements: self.up_to_date_elements,
            outdated_elements: self.outdated_elements,
            legacy_elements: self.legacy_elements,
            elements_created: self.elements_created,
            elements_updated: self.elements_updated,
            elements_deleted: self.elements_deleted,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

#[get("/v2/reports")]
async fn get(conn: Data<Mutex<Connection>>) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .prepare(db::REPORT_SELECT_ALL)?
            .query_map([], db::mapper_daily_report_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
    ))
}

#[get("/v2/reports/{id}")]
pub async fn get_by_id(
    id: Path<String>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Option<GetItem>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .query_row(
                db::REPORT_SELECT_BY_ID,
                [id.into_inner()],
                db::mapper_daily_report_full(),
            )
            .optional()?
            .map(|it| it.into()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use actix_web::test::TestRequest;
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
                .service(super::get),
        )
        .await;
        let req = TestRequest::get().uri("/v2/reports").to_request();
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
            db::REPORT_INSERT,
            named_params! {
                ":area_id" : "",
                ":date" : "",
                ":total_elements" : 0,
                ":total_elements_onchain" : 0,
                ":total_elements_lightning" : 0,
                ":total_elements_lightning_contactless" : 0,
                ":up_to_date_elements" : 0,
                ":outdated_elements" : 0,
                ":legacy_elements" : 0,
                ":elements_created" : 0,
                ":elements_updated" : 0,
                ":elements_deleted" : 0,
            },
        )
        .unwrap();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(Mutex::new(db)))
                .service(super::get),
        )
        .await;
        let req = TestRequest::get().uri("/v2/reports").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 1);
    }
}
