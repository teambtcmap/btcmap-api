use crate::db;
use crate::model::ApiError;
use crate::model::DailyReport;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use rusqlite::Connection;
use serde::Serialize;
use std::sync::Mutex;

#[derive(Serialize)]
pub struct GetReportItem {
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
}

impl Into<GetReportItem> for DailyReport {
    fn into(self) -> GetReportItem {
        GetReportItem {
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
        }
    }
}

#[derive(Serialize)]
pub struct GetReportItemV2 {
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

impl Into<GetReportItemV2> for DailyReport {
    fn into(self) -> GetReportItemV2 {
        GetReportItemV2 {
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

#[get("/daily_reports")]
async fn get(conn: Data<Mutex<Connection>>) -> Result<Json<Vec<GetReportItem>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .prepare(db::DAILY_REPORT_SELECT_ALL)?
            .query_map([], db::mapper_daily_report_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
    ))
}

#[get("/v2/reports")]
async fn get_v2(conn: Data<Mutex<Connection>>) -> Result<Json<Vec<GetReportItemV2>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .prepare(db::DAILY_REPORT_SELECT_ALL)?
            .query_map([], db::mapper_daily_report_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
    ))
}
