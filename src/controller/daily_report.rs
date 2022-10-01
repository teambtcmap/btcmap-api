use crate::model::ApiError;
use crate::model::DailyReport;
use crate::db;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use rusqlite::Connection;
use std::sync::Mutex;

#[get("/daily_reports")]
async fn get_daily_reports(
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Vec<DailyReport>>, ApiError> {
    Ok(Json(
        conn.lock()?
            .prepare(db::DAILY_REPORT_SELECT_ALL)?
            .query_map([], db::mapper_daily_report_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap())
            .collect(),
    ))
}
