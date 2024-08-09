use crate::element::service as element_service;
use crate::{area::Area, command::db::open_connection, Result};
use tracing::info;

// Example: btcmap-api remove-areas-tag th
pub fn run(args: Vec<String>) -> Result<()> {
    let area_id_or_alias = args.get(2).unwrap();
    info!(area_id_or_alias);
    let mut conn = open_connection()?;
    let area = Area::select_by_id_or_alias(area_id_or_alias, &conn)?.unwrap();
    info!(area.id, area_name = area.name(), area_alias = area.alias());
    element_service::remove_areas_tag(&area, &mut conn)?;
    Ok(())
}
