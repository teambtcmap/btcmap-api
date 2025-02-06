use crate::admin::Admin;
use crate::conf::Conf;
use crate::osm::overpass;
use crate::sync::MergeResult;
use crate::{db, discord, sync, Result};
use serde::Serialize;

#[derive(Serialize)]
pub struct Res {
    pub overpass_query_time_s: f64,
    pub overpass_elements: usize,
    pub merge_result: MergeResult,
}

pub async fn run(admin: &Admin, conf: &Conf) -> Result<Res> {
    let overpass_res = overpass::query_bitcoin_merchants().await?;
    let overpass_elements_len = overpass_res.elements.len();
    let mut conn = db::open_connection()?;
    let merge_res = sync::merge_overpass_elements(overpass_res.elements, &mut conn).await?;
    if merge_res.elements_created.len()
        + merge_res.elements_updated.len()
        + merge_res.elements_deleted.len()
        > 5
    {
        discord::post_message(
            &conf.discord_webhook_api,
            format!(
                "Admin {} ran a sync with a high number of changes (created: {}, updated: {}, deleted: {})",
                admin.name,
                merge_res.elements_created.len(),
                merge_res.elements_updated.len(),
                merge_res.elements_deleted.len()
            )
        ).await;
    }
    Ok(Res {
        overpass_query_time_s: overpass_res.time_s,
        overpass_elements: overpass_elements_len,
        merge_result: merge_res,
    })
}
