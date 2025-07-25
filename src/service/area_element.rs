use crate::db::area::schema::Area;
use crate::db::element::schema::Element;
use crate::{db, service, Result};
use deadpool_sqlite::Pool;
use geo::{Contains, LineString, MultiPolygon, Polygon};
use geojson::Geometry;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Diff {
    pub element_id: i64,
    pub element_osm_url: String,
    pub added_areas: Vec<i64>,
    pub removed_areas: Vec<i64>,
}

pub async fn generate_mapping(elements: &[Element], pool: &Pool) -> Result<Vec<Diff>> {
    let mut diffs = vec![];
    let all_areas = db::area::queries_async::select(None, true, None, pool).await?;
    for element in elements {
        if let Some(diff) = generate_element_areas_mapping(element, &all_areas, pool).await? {
            diffs.push(diff);
        }
    }
    Ok(diffs)
}

pub async fn generate_element_areas_mapping(
    element: &Element,
    areas: &Vec<Area>,
    pool: &Pool,
) -> Result<Option<Diff>> {
    let mut added_areas: Vec<i64> = vec![];
    let mut removed_areas: Vec<i64> = vec![];
    let old_mappings =
        db::area_element::queries_async::select_by_element_id(element.id, pool).await?;
    let new_mappings = service::element::find_areas(&element, &areas)?;
    // mark no longer active mappings as deleted
    for old_mapping in &old_mappings {
        if old_mapping.deleted_at.is_none() {
            let still_valid = new_mappings
                .iter()
                .find(|area| area.id == old_mapping.area_id)
                .is_some();
            if !still_valid {
                db::area_element::queries_async::set_deleted_at(
                    old_mapping.id,
                    Some(OffsetDateTime::now_utc()),
                    pool,
                )
                .await?;
                removed_areas.push(old_mapping.area_id);
            }
        }
    }
    // refresh data to include the changes made above
    let old_mappings =
        db::area_element::queries_async::select_by_element_id(element.id, pool).await?;
    for area in new_mappings {
        let old_mapping = old_mappings
            .iter()
            .find(|old_mapping| old_mapping.area_id == area.id);
        match old_mapping {
            Some(old_mapping) => {
                if old_mapping.deleted_at.is_some() {
                    db::area_element::queries_async::set_deleted_at(old_mapping.id, None, pool)
                        .await?;
                    added_areas.push(area.id);
                }
            }
            None => {
                db::area_element::queries_async::insert(area.id, element.id, pool).await?;
                added_areas.push(area.id);
            }
        }
    }
    let res = if !added_areas.is_empty() || !removed_areas.is_empty() {
        Some(Diff {
            element_id: element.id,
            element_osm_url: element.osm_url(),
            added_areas,
            removed_areas,
        })
    } else {
        None
    };
    Ok(res)
}

pub async fn get_elements_within_geometries(
    geometries: Vec<Geometry>,
    pool: &Pool,
) -> Result<Vec<Element>> {
    let mut area_elements: Vec<Element> = vec![];
    for element in db::element::queries_async::select_updated_since(
        OffsetDateTime::UNIX_EPOCH,
        None,
        true,
        pool,
    )
    .await?
    {
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
