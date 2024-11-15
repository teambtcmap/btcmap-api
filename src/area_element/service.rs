use super::model::AreaElement;
use crate::{area::Area, element::Element};
use crate::{element, Result};
use geo::{Contains, LineString, MultiPolygon, Polygon};
use geojson::Geometry;
use rusqlite::Connection;
use time::OffsetDateTime;

pub struct Res {
    pub has_changes: bool,
}

pub fn generate_mapping(elements: &[Element], conn: &Connection) -> Result<Res> {
    let mut has_changes = false;
    let areas = Area::select_all(conn)?;
    for element in elements {
        let element_areas = element::service::find_areas(element, &areas)?;
        let old_mappings = AreaElement::select_by_element_id(element.id, conn)?;
        let old_mappings: Vec<AreaElement> = old_mappings
            .into_iter()
            .filter(|it| it.deleted_at.is_none())
            .collect();
        let mut old_area_ids: Vec<i64> = old_mappings.into_iter().map(|it| it.area_id).collect();
        let mut new_area_ids: Vec<i64> = element_areas.into_iter().map(|it| it.id).collect();
        old_area_ids.sort();
        new_area_ids.sort();
        if new_area_ids != old_area_ids {
            for old_area_id in &old_area_ids {
                if !new_area_ids.contains(old_area_id) {
                    let area_element = AreaElement::select_by_area_id_and_element_id(
                        *old_area_id,
                        element.id,
                        conn,
                    )?
                    .unwrap();
                    AreaElement::set_deleted_at(
                        area_element.id,
                        Some(OffsetDateTime::now_utc()),
                        conn,
                    )?;
                }
            }
            for new_area_id in new_area_ids {
                if !old_area_ids.contains(&new_area_id) {
                    let area_element = AreaElement::select_by_area_id_and_element_id(
                        new_area_id,
                        element.id,
                        conn,
                    )?;
                    match area_element {
                        Some(area_element) => {
                            AreaElement::set_deleted_at(area_element.id, None, conn)?;
                        }
                        None => {
                            AreaElement::insert(new_area_id, element.id, conn)?;
                        }
                    }
                }
            }
            has_changes = true;
        }
    }
    Ok(Res { has_changes })
}

pub fn get_elements_within_geometries(
    geometries: Vec<Geometry>,
    conn: &Connection,
) -> Result<Vec<Element>> {
    let mut area_elements: Vec<Element> = vec![];
    for element in Element::select_all(None, conn)? {
        for geometry in &geometries {
            match &geometry.value {
                geojson::Value::MultiPolygon(_) => {
                    let multi_poly: MultiPolygon = (&geometry.value).try_into().unwrap();

                    if multi_poly.contains(&element.overpass_data.coord()) {
                        area_elements.push(element.clone());
                    }
                }
                geojson::Value::Polygon(_) => {
                    let poly: Polygon = (&geometry.value).try_into().unwrap();

                    if poly.contains(&element.overpass_data.coord()) {
                        area_elements.push(element.clone());
                    }
                }
                geojson::Value::LineString(_) => {
                    let line_string: LineString = (&geometry.value).try_into().unwrap();

                    if line_string.contains(&element.overpass_data.coord()) {
                        area_elements.push(element.clone());
                    }
                }
                _ => continue,
            }
        }
    }

    Ok(area_elements)
}
