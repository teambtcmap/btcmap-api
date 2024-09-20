use crate::osm::overpass::query_bitcoin_merchants;
use crate::osm::overpass::OverpassElement;
use crate::sync;
use crate::Result;
use rusqlite::Connection;
use std::collections::HashSet;
use tracing::info;

pub async fn run(conn: &mut Connection) -> Result<()> {
    let elements = query_bitcoin_merchants().await?;
    let res = merge_overpass_elements(elements, conn).await?;
    info!(res.elements_updated, res.elements_deleted);
    Ok(())
}

pub struct MergeResult {
    pub elements_updated: usize,
    pub elements_deleted: usize,
}

async fn merge_overpass_elements(
    fresh_overpass_elements: Vec<OverpassElement>,
    conn: &mut Connection,
) -> Result<MergeResult> {
    // stage 1: find and process deleted elements
    let fresh_elemement_ids: HashSet<String> = fresh_overpass_elements
        .iter()
        .map(|it| it.btcmap_id())
        .collect();
    let deleted_element_events = sync::sync_deleted_elements(&fresh_elemement_ids, conn).await?;
    // stage 2: find and process updated elements
    let updated_element_events =
        sync::sync_updated_elements(&fresh_overpass_elements, conn).await?;
    // stage 3: find and process new elements
    sync::sync_new_elements(&fresh_overpass_elements, conn).await?;
    Ok(MergeResult {
        elements_updated: updated_element_events.len(),
        elements_deleted: deleted_element_events.len(),
    })
}
