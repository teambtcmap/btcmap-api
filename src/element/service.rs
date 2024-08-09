use super::Element;
use crate::area::Area;
use crate::Result;
use geo::Contains;
use geo::LineString;
use geo::MultiPolygon;
use geo::Polygon;
use rusqlite::Connection;
use serde_json::json;
use serde_json::Value;
use tracing::info;

pub fn remove_areas_tag(area: &Area, conn: &mut Connection) -> Result<()> {
    let sp = conn.savepoint()?;
    info!(
        area.id,
        alias = area.alias(),
        "Removing areas tag from area {} ({})",
        area.id,
        area.name(),
    );
    let area_elements = find_in_area(area, &sp)?;
    info!(
        count = area_elements.len(),
        "Found {} elements in {}",
        area_elements.len(),
        area.name(),
    );
    for area_element in area_elements {
        area_element.remove_tag("areas", &sp)?;
    }
    sp.commit()?;
    Ok(())
}

pub fn find_in_area(area: &Area, conn: &Connection) -> Result<Vec<Element>> {
    let all_elements: Vec<Element> = Element::select_all(None, &conn)?
        .into_iter()
        .filter(|it| it.deleted_at == None)
        .collect();
    filter_by_area(&all_elements, area)
}

pub fn filter_by_area(all_elements: &Vec<Element>, area: &Area) -> Result<Vec<Element>> {
    let geometries = area.geo_json_geometries();
    let mut area_elements: Vec<Element> = vec![];

    for element in all_elements {
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

pub fn update_areas_tag(elements: &Vec<Element>, conn: &Connection) -> Result<Vec<Element>> {
    let mut res: Vec<Element> = vec![];

    let all_areas: Vec<Area> = Area::select_all(None, &conn)?
        .into_iter()
        .filter(|it| it.deleted_at == None)
        .collect();

    for element in elements {
        let element_areas = find_areas(element, &all_areas)?;
        let element_areas = areas_to_areas_tag(element_areas);

        let element = if element.tag("areas") != &element_areas {
            info!(
                element = element.id,
                old = serde_json::to_string(element.tag("areas"))?,
                new = serde_json::to_string(&element_areas)?,
                "Change detected, updating areas tag",
            );
            element.set_tag("areas", &element_areas, conn)?
        } else {
            info!(element = element.id, "No changes, skipping update");
            element.clone()
        };

        res.push(element);
    }

    Ok(res)
}

fn areas_to_areas_tag(areas: Vec<&Area>) -> Value {
    let element_areas: Vec<Value> = areas.iter().map(|it| {
        json!({"id": it.id, "url_alias": it.tags.get("url_alias").unwrap_or(&Value::Null).as_str().unwrap_or_default()})
    }).collect();
    Value::Array(element_areas)
}

pub fn find_areas<'a>(element: &Element, areas: &'a Vec<Area>) -> Result<Vec<&'a Area>> {
    let mut element_areas = vec![];

    for area in areas {
        if area.tags.get("url_alias") == Some(&Value::String("earth".into())) {
            continue;
        }

        let geometries = area.geo_json_geometries();

        for geometry in &geometries {
            match &geometry.value {
                geojson::Value::MultiPolygon(_) => {
                    let multi_poly: MultiPolygon = (&geometry.value).try_into().unwrap();

                    if multi_poly.contains(&element.overpass_data.coord()) {
                        element_areas.push(area);
                    }
                }
                geojson::Value::Polygon(_) => {
                    let poly: Polygon = (&geometry.value).try_into().unwrap();

                    if poly.contains(&element.overpass_data.coord()) {
                        element_areas.push(area);
                    }
                }
                geojson::Value::LineString(_) => {
                    let line_string: LineString = (&geometry.value).try_into().unwrap();

                    if line_string.contains(&element.overpass_data.coord()) {
                        element_areas.push(area);
                    }
                }
                _ => continue,
            }
        }
    }

    Ok(element_areas)
}
