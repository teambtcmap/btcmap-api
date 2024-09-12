use super::model::AreaElement;
use crate::{area::Area, element::Element};
use crate::{element, Result};
use rusqlite::Connection;
use time::OffsetDateTime;

pub struct Res {
    pub has_changes: bool,
}

pub fn generate_areas_mapping(
    element: &Element,
    areas: &Vec<Area>,
    conn: &Connection,
) -> Result<Res> {
    let mut has_changes = false;
    let element_areas = element::service::find_areas(&element, &areas)?;
    let old_mappings = AreaElement::select_by_element_id(element.id, conn)?;
    let mut old_area_ids: Vec<i64> = old_mappings.into_iter().map(|it| it.area_id).collect();
    let mut new_area_ids: Vec<i64> = element_areas.into_iter().map(|it| it.id).collect();
    old_area_ids.sort();
    new_area_ids.sort();
    if new_area_ids != old_area_ids {
        for old_area_id in &old_area_ids {
            if !new_area_ids.contains(&old_area_id) {
                AreaElement::set_deleted_at(*old_area_id, Some(OffsetDateTime::now_utc()), conn)?;
            }
        }
        for new_area_id in new_area_ids {
            if !old_area_ids.contains(&new_area_id) {
                AreaElement::insert(new_area_id, element.id, conn)?;
            }
        }
        has_changes = true;
    }
    Ok(Res { has_changes })
}
