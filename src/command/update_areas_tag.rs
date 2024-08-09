use crate::command::db::open_connection;
use crate::element;
use crate::{area::Area, Result};
use tracing::info;

// Example: btcmap-api update-areas-tag th
pub fn run(args: Vec<String>) -> Result<()> {
    let area_id_or_alias = args.get(2).unwrap();
    info!(area_id_or_alias);
    let conn = open_connection()?;
    let area = Area::select_by_id_or_alias(area_id_or_alias, &conn)?.unwrap();
    info!(area.id, area_name = area.name(), area_alias = area.alias());
    let area_elements = element::service::find_in_area(&area, &conn)?;
    element::service::update_areas_tag(&area_elements, &conn)?;
    Ok(())
}
