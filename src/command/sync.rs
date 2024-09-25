use crate::osm::overpass::query_bitcoin_merchants;
use crate::sync;
use crate::Result;
use rusqlite::Connection;
use tracing::info;

pub async fn run(conn: &mut Connection) -> Result<()> {
    let elements = query_bitcoin_merchants().await?;
    let res = sync::merge_overpass_elements(elements, conn).await?;
    info!(res.elements_updated, res.elements_deleted);
    Ok(())
}
