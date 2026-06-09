use crate::db::main::area::schema::Area;
use crate::db::main::element::schema::Element;
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
    let all_areas = db::main::area::queries::select(None, true, None, pool).await?;
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
        db::main::area_element::queries::select_by_element_id(element.id, pool).await?;
    let new_mappings = service::element::find_areas(element, areas)?;
    // mark no longer active mappings as deleted
    for old_mapping in &old_mappings {
        if old_mapping.deleted_at.is_none() {
            let still_valid = new_mappings
                .iter()
                .any(|area| area.id == old_mapping.area_id);

            if !still_valid {
                db::main::area_element::queries::set_deleted_at(
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
        db::main::area_element::queries::select_by_element_id(element.id, pool).await?;
    for area in new_mappings {
        let old_mapping = old_mappings
            .iter()
            .find(|old_mapping| old_mapping.area_id == area.id);
        match old_mapping {
            Some(old_mapping) => {
                if old_mapping.deleted_at.is_some() {
                    db::main::area_element::queries::set_deleted_at(old_mapping.id, None, pool)
                        .await?;
                    added_areas.push(area.id);
                }
            }
            None => {
                db::main::area_element::queries::insert(area.id, element.id, pool).await?;
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
    for element in db::main::element::queries::select_updated_since(
        OffsetDateTime::UNIX_EPOCH,
        None,
        true,
        pool,
    )
    .await?
    {
        for geometry in &geometries {
            match &geometry.value {
                geojson::GeometryValue::MultiPolygon { coordinates: _ } => {
                    let multi_poly: MultiPolygon = (&geometry.value).try_into().unwrap();

                    if multi_poly.contains(&element.overpass_data.coord()) {
                        area_elements.push(element.clone());
                    }
                }
                geojson::GeometryValue::Polygon { coordinates: _ } => {
                    let poly: Polygon = (&geometry.value).try_into().unwrap();

                    if poly.contains(&element.overpass_data.coord()) {
                        area_elements.push(element.clone());
                    }
                }
                geojson::GeometryValue::LineString { coordinates: _ } => {
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

#[cfg(test)]
mod test {
    use super::get_elements_within_geometries;
    use crate::db::main::element::queries as element_queries;
    use crate::db::main::test::pool;
    use crate::service::overpass::OverpassElement;
    use crate::Result;
    use actix_web::test;
    use deadpool_sqlite::Pool;
    use geojson::Geometry;

    async fn insert_element(pool: &Pool, id: i64, lat: f64, lon: f64) -> Result<()> {
        let element = OverpassElement {
            lat: Some(lat),
            lon: Some(lon),
            ..OverpassElement::mock(id)
        };
        element_queries::insert(element, pool).await?;
        Ok(())
    }

    #[test]
    async fn get_elements_within_polygon() -> Result<()> {
        let pool = pool();
        insert_element(&pool, 1, 7.979, 98.334).await?;
        let ring: Vec<[f64; 2]> = vec![
            [98.21, 8.20],
            [98.21, 7.74],
            [98.48, 7.74],
            [98.48, 8.20],
            [98.21, 8.20],
        ];
        let geometry = Geometry::new_polygon([ring]);
        let hits = get_elements_within_geometries(vec![geometry], &pool).await?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, 1);
        Ok(())
    }

    #[test]
    async fn get_elements_within_multi_polygon() -> Result<()> {
        let pool = pool();
        insert_element(&pool, 1, 10.0, 10.0).await?;
        let polygons: Vec<Vec<Vec<[f64; 2]>>> = vec![
            vec![vec![
                [-1.0, -1.0],
                [-1.0, 1.0],
                [1.0, 1.0],
                [1.0, -1.0],
                [-1.0, -1.0],
            ]],
            vec![vec![
                [9.0, 9.0],
                [9.0, 11.0],
                [11.0, 11.0],
                [11.0, 9.0],
                [9.0, 9.0],
            ]],
        ];
        let geometry = Geometry::new_multi_polygon(polygons);
        let hits = get_elements_within_geometries(vec![geometry], &pool).await?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, 1);
        Ok(())
    }

    #[test]
    async fn get_elements_within_line_string() -> Result<()> {
        let pool = pool();
        insert_element(&pool, 1, 2.5, 2.5).await?;
        let line: Vec<[f64; 2]> = vec![[0.0, 0.0], [5.0, 5.0]];
        let geometry = Geometry::new_line_string(line);
        let hits = get_elements_within_geometries(vec![geometry], &pool).await?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, 1);
        Ok(())
    }

    #[test]
    async fn get_elements_excludes_points_outside_geometry() -> Result<()> {
        let pool = pool();
        insert_element(&pool, 1, 50.0, 1.0).await?;
        let ring: Vec<[f64; 2]> = vec![
            [98.21, 8.20],
            [98.21, 7.74],
            [98.48, 7.74],
            [98.48, 8.20],
            [98.21, 8.20],
        ];
        let geometry = Geometry::new_polygon([ring]);
        let hits = get_elements_within_geometries(vec![geometry], &pool).await?;
        assert!(hits.is_empty());
        Ok(())
    }
}
