use crate::db::log::sync::blocking_queries::UpdateArgs;
use crate::db::log::sync::blocking_queries::UpdateFailedArgs;
use crate::db::log::sync::queries as sync_log_queries;
use crate::db::log::LogPool;
use crate::service::sync::MergeResult;
use crate::service::{self, matrix};
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Res {
    pub overpass_query_time_s: f64,
    pub overpass_elements: usize,
    pub merge_result: MergeResult,
}

pub async fn run(pool: &Pool, log_pool: &LogPool) -> Result<Res> {
    let started_at = OffsetDateTime::now_utc();
    let sync_log_id = sync_log_queries::insert(log_pool).await?;

    let overpass_res = match service::overpass::query_bitcoin_merchants().await {
        Ok(res) => res,
        Err(e) => {
            let failed_at = OffsetDateTime::now_utc().format(&Rfc3339).unwrap();
            let fail_reason = e.to_string();
            sync_log_queries::update_failed(
                UpdateFailedArgs {
                    id: sync_log_id,
                    failed_at,
                    fail_reason,
                },
                log_pool,
            )
            .await?;
            return Err(e);
        }
    };
    let overpass_elements_len = overpass_res.elements.len();
    let matrix_client = matrix::try_client(pool);
    let merge_res =
        match service::sync::merge_overpass_elements(overpass_res.elements, pool, &matrix_client)
            .await
        {
            Ok(res) => res,
            Err(e) => {
                let failed_at = OffsetDateTime::now_utc().format(&Rfc3339).unwrap();
                let fail_reason = e.to_string();
                sync_log_queries::update_failed(
                    UpdateFailedArgs {
                        id: sync_log_id,
                        failed_at,
                        fail_reason,
                    },
                    log_pool,
                )
                .await?;
                return Err(e);
            }
        };

    let finished_at = OffsetDateTime::now_utc().format(&Rfc3339).unwrap();
    let duration_s = (OffsetDateTime::now_utc() - started_at).as_seconds_f64();
    let elements_created = merge_res.elements_created.len() as i64;
    let elements_updated = merge_res.elements_updated.len() as i64;
    let elements_deleted = merge_res.elements_deleted.len() as i64;

    let args = UpdateArgs {
        id: sync_log_id,
        finished_at,
        duration_s,
        overpass_response_time_s: overpass_res.time_s,
        elements_created,
        elements_updated,
        elements_deleted,
    };

    sync_log_queries::update_completed(args, log_pool).await?;

    Ok(Res {
        overpass_query_time_s: overpass_res.time_s,
        overpass_elements: overpass_elements_len,
        merge_result: merge_res,
    })
}
