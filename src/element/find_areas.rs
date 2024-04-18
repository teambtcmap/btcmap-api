use super::Element;
use crate::{area::Area, Result};
use geo::Contains;
use geo::LineString;
use geo::MultiPolygon;
use geo::Polygon;
use geojson::{GeoJson, Geometry};
use rusqlite::Connection;
use serde_json::json;
use serde_json::Value;
use tracing::error;
use tracing::info;

pub fn run(conn: &Connection) -> Result<()> {
    info!("Loading elements");
    let elements: Vec<Element> = Element::select_all(None, &conn)?
        .into_iter()
        .filter(|it| it.deleted_at.is_none())
        .collect();

    info!("Loading areas");
    let areas: Vec<Area> = Area::select_all(None, &conn)?
        .into_iter()
        .filter(|it| it.deleted_at == None)
        .collect();

    let mut counter = 1;

    for element in &elements {
        info!("Processing element {} of {}", counter, elements.len());
        find_and_save(element, &areas, conn)?;
        counter = counter + 1;
    }

    Ok(())
}

pub fn find_and_save(element: &Element, areas: &Vec<Area>, conn: &Connection) -> Result<()> {
    let element_areas = find_areas(element, &areas)?;
    let element_area_names: Vec<&str> = element_areas
        .iter()
        .map(|it| {
            it.tags
                .get("name")
                .unwrap_or(&Value::Null)
                .as_str()
                .unwrap_or("Unnamed")
        })
        .collect();
    info!(
            element = element.overpass_data.tag("name"),
            areas = ?element_area_names,
    );

    let element_areas: Vec<Value> = element_areas.iter().map(|it| {
        json!({"id": it.id, "url_alias": it.tags.get("url_alias").unwrap_or(&Value::Null).as_str().unwrap_or_default()})
    }).collect();

    let element_areas = Value::Array(element_areas);
    let old_element_areas = element.tag("areas");

    if old_element_areas != &element_areas {
        info!("Change detected, updating areas tag");
        element.set_tag("areas", &element_areas, conn)?;
    } else {
        info!("No changes, skipping update");
    }

    Ok(())
}

fn find_areas<'a>(element: &Element, areas: &'a Vec<Area>) -> Result<Vec<&'a Area>> {
    let mut element_areas = vec![];

    for area in areas {
        if area.tags.get("url_alias") == Some(&Value::String("earth".into())) {
            continue;
        }

        let geo_json = area.tags.get("geo_json").unwrap_or(&Value::Null);

        if geo_json.is_null() {
            continue;
        }

        if geo_json.is_object() {
            let geo_json: Result<GeoJson, _> = serde_json::to_string(geo_json)?.parse();

            let geo_json = match geo_json {
                Ok(geo_json) => geo_json,
                Err(e) => {
                    error!(?e, "Failed to parse GeoJSON");
                    continue;
                }
            };

            let mut geometries: Vec<&Geometry> = vec![];

            match &geo_json {
                GeoJson::FeatureCollection(v) => {
                    for feature in &v.features {
                        if let Some(v) = &feature.geometry {
                            geometries.push(v);
                        }
                    }
                }
                GeoJson::Feature(v) => {
                    if let Some(v) = &v.geometry {
                        geometries.push(v);
                    }
                }
                GeoJson::Geometry(v) => geometries.push(v),
            };

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
    }

    Ok(element_areas)
}
