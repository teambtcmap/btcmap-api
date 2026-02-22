use crate::service::sync::MergeResult;
use crate::service::{self, matrix};
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Serialize;

#[derive(Serialize)]
pub struct Res {
    pub overpass_query_time_s: f64,
    pub overpass_elements: usize,
    pub merge_result: MergeResult,
}

pub async fn run(pool: &Pool) -> Result<Res> {
    let overpass_res = service::overpass::query_bitcoin_merchants().await?;
    let overpass_elements_len = overpass_res.elements.len();
    let matrix_client = matrix::try_client(pool);
    let merge_res =
        service::sync::merge_overpass_elements(overpass_res.elements, pool, &matrix_client).await?;
    Ok(Res {
        overpass_query_time_s: overpass_res.time_s,
        overpass_elements: overpass_elements_len,
        merge_result: merge_res,
    })
}
