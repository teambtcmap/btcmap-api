use crate::db::conf::schema::Conf;
use crate::db::user::schema::User;
use crate::service::sync::MergeResult;
use crate::service::{self, discord};
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Serialize;

#[derive(Serialize)]
pub struct Res {
    pub overpass_query_time_s: f64,
    pub overpass_elements: usize,
    pub merge_result: MergeResult,
}

pub async fn run(user: &User, pool: &Pool, conf: &Conf) -> Result<Res> {
    let overpass_res = service::overpass::query_bitcoin_merchants().await?;
    let overpass_elements_len = overpass_res.elements.len();
    let merge_res = service::sync::merge_overpass_elements(overpass_res.elements, pool).await?;
    if merge_res.elements_created.len()
        + merge_res.elements_updated.len()
        + merge_res.elements_deleted.len()
        > 5
    {
        discord::send(
            format!(
                "{} ran a sync with a high number of changes (created: {}, updated: {}, deleted: {})",
                user.name,
                merge_res.elements_created.len(),
                merge_res.elements_updated.len(),
                merge_res.elements_deleted.len()
            ),
            discord::Channel::Api,
            conf,
        );
    }
    Ok(Res {
        overpass_query_time_s: overpass_res.time_s,
        overpass_elements: overpass_elements_len,
        merge_result: merge_res,
    })
}
