use crate::{
    db::{self, main::area::schema::Area},
    Result,
};
use deadpool_sqlite::Pool;
use geo::Contains;
use geo::LineString;
use geo::MultiPolygon;
use geo::Polygon;

/// Verify that the (lat, lon) the caller wants to operate on is inside the
/// caller's geofence when the geofence is non-empty.
///
/// The geofence is a set of area polygons. The check is purely geometric:
/// the point must fall inside at least one of them. `area_id` is intentionally
/// ignored so that sub-areas (e.g. a community inside a country-level fence)
/// are accepted as long as the point itself is inside the fenced region.
///
/// Returns `Ok(true)` when no check is needed because the geofence is empty or
/// when the point falls inside the fence.
///
/// Returns an error when the caller has a non-empty geofence and the point
/// falls outside every fenced area.
pub(crate) async fn check(
    user: &crate::db::main::user::schema::User,
    lat: f64,
    lon: f64,
    pool: &Pool,
) -> Result<bool> {
    if user.geofence.is_empty() {
        return Ok(true);
    }

    let areas = db::main::area::queries::select_by_ids(&user.geofence, pool).await?;
    let coord = geo::coord!(x: lon, y: lat);
    for area in &areas {
        if point_inside_area(area, coord)? {
            return Ok(true);
        }
    }

    Err(format!(
        "Location ({lat}, {lon}) is outside your geofence (allowed areas: {:?})",
        user.geofence
    )
    .into())
}

fn point_inside_area(area: &Area, coord: geo::Coord) -> Result<bool> {
    let geometries = area.geo_json_geometries()?;
    for geometry in &geometries {
        match &geometry.value {
            geojson::GeometryValue::Polygon { coordinates: _ } => {
                let poly: Polygon = (&geometry.value).try_into().unwrap();
                if poly.contains(&coord) {
                    return Ok(true);
                }
            }
            geojson::GeometryValue::MultiPolygon { coordinates: _ } => {
                let multi_poly: MultiPolygon = (&geometry.value).try_into().unwrap();
                if multi_poly.contains(&coord) {
                    return Ok(true);
                }
            }
            geojson::GeometryValue::LineString { coordinates: _ } => {
                let line_string: LineString = (&geometry.value).try_into().unwrap();
                if line_string.contains(&coord) {
                    return Ok(true);
                }
            }
            _ => continue,
        }
    }
    Ok(false)
}

/// Convenience wrapper used by `delete_event`: load the existing event
/// then run the geofence check against its stored (lat, lon).
pub(crate) async fn check_existing(
    user: &crate::db::main::user::schema::User,
    event_id: i64,
    pool: &Pool,
) -> Result<()> {
    let event = db::main::event::queries::select_by_id(event_id, pool).await?;
    check(user, event.lat, event.lon, pool).await.map(|_| ())
}

#[cfg(test)]
mod test {
    use super::check;
    use crate::{
        db,
        db::main::{
            area::schema::Area,
            test::pool,
            user::schema::{Role, User},
        },
        Result,
    };
    use serde_json::{json, Map};

    // A small Phuket-shaped polygon used as the geofenced area.
    const PHUKET: &str = r#"{
        "type":"Feature",
        "properties":{},
        "geometry":{
            "type":"Polygon",
            "coordinates":[[
                [98.2181205776469, 8.20412838698085],
                [98.2181205776469, 7.74024270965898],
                [98.4806081271079, 7.74024270965898],
                [98.4806081271079, 8.20412838698085],
                [98.2181205776469, 8.20412838698085]
            ]]
        }
    }"#;

    // A separate polygon in central London (way outside Phuket).
    const LONDON: &str = r#"{
        "type":"Feature",
        "properties":{},
        "geometry":{
            "type":"Polygon",
            "coordinates":[[
                [-0.2, 51.45],
                [-0.2, 51.55],
                [ 0.0, 51.55],
                [ 0.0, 51.45],
                [-0.2, 51.45]
            ]]
        }
    }"#;

    async fn insert_area(
        name: &str,
        geo_json: serde_json::Value,
        pool: &deadpool_sqlite::Pool,
    ) -> Result<Area> {
        let mut tags = Map::new();
        tags.insert("name".into(), json!(name));
        tags.insert("geo_json".into(), geo_json);
        tags.insert("url_alias".into(), json!(name));
        db::main::area::queries::insert(tags, pool).await
    }

    fn user_with(roles: Vec<Role>, geofence: Vec<i64>) -> User {
        User {
            id: 1,
            name: "tester".into(),
            password: String::new(),
            roles,
            saved_places: vec![],
            saved_areas: vec![],
            npub: None,
            geofence,
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
        }
    }

    #[test]
    fn roles_with_non_empty_geofence_are_restricted() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            for role in [Role::Root, Role::Admin, Role::EventManager] {
                let user = user_with(vec![role], vec![1]);
                // Point (0, 0) is null island, definitely outside area id=1
                // (which is the test's first insert and has no polygon set).
                let err = check(&user, 0.0, 0.0, &pool).await.unwrap_err();
                assert!(err.to_string().contains("outside your geofence"));
            }
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn event_manager_with_empty_geofence_is_unconstrained() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let user = user_with(vec![Role::EventManager], vec![]);
            assert!(check(&user, 0.0, 0.0, &pool).await?);
            assert!(check(&user, 51.5, -0.1, &pool).await?);
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn event_manager_with_geofence_allows_point_inside_polygon() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let phuket =
                insert_area("phuket", serde_json::from_str(PHUKET).unwrap(), &pool).await?;
            let user = user_with(vec![Role::EventManager], vec![phuket.id]);
            // Inside Phuket polygon
            assert!(check(&user, 7.98, 98.33, &pool).await?);
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn event_manager_with_geofence_rejects_point_outside_polygon() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let phuket =
                insert_area("phuket", serde_json::from_str(PHUKET).unwrap(), &pool).await?;
            let user = user_with(vec![Role::EventManager], vec![phuket.id]);
            // Inside London polygon, way outside Phuket
            let err = check(&user, 51.5, -0.1, &pool).await.unwrap_err();
            assert!(err.to_string().contains("outside your geofence"));
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn event_manager_with_multiple_areas_in_geofence() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let phuket =
                insert_area("phuket", serde_json::from_str(PHUKET).unwrap(), &pool).await?;
            let london =
                insert_area("london", serde_json::from_str(LONDON).unwrap(), &pool).await?;
            let user = user_with(vec![Role::EventManager], vec![phuket.id, london.id]);
            // Phuket point passes
            assert!(check(&user, 7.98, 98.33, &pool).await?);
            // London point passes
            assert!(check(&user, 51.5, -0.1, &pool).await?);
            // In-between point fails
            let err = check(&user, 40.0, 50.0, &pool).await.unwrap_err();
            assert!(err.to_string().contains("outside your geofence"));
            Ok::<(), crate::Error>(())
        })
    }

    // A small polygon that lives entirely inside the Phuket polygon above.
    // Used to model a sub-area (e.g. a community inside a country-level fence)
    // and verify the geofence check doesn't care about area ids.
    const PHUKET_SUB: &str = r#"{
        "type":"Feature",
        "properties":{},
        "geometry":{
            "type":"Polygon",
            "coordinates":[[
                [98.30, 7.95],
                [98.30, 7.85],
                [98.40, 7.85],
                [98.40, 7.95],
                [98.30, 7.95]
            ]]
        }
    }"#;

    #[test]
    fn point_inside_subarea_is_allowed_by_parent_geofence() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let phuket =
                insert_area("phuket", serde_json::from_str(PHUKET).unwrap(), &pool).await?;
            // The sub-area exists in the database but is NOT in the user's
            // geofence. The check must still pass because the point is
            // geometrically inside the fenced (parent) area.
            let _subarea = insert_area(
                "phuket-tourism",
                serde_json::from_str(PHUKET_SUB).unwrap(),
                &pool,
            )
            .await?;
            let user = user_with(vec![Role::EventManager], vec![phuket.id]);
            assert!(check(&user, 7.90, 98.35, &pool).await?);
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn point_outside_parent_geofence_is_rejected_even_if_subarea_exists() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let phuket =
                insert_area("phuket", serde_json::from_str(PHUKET).unwrap(), &pool).await?;
            let subarea = insert_area(
                "some-other",
                serde_json::from_str(PHUKET_SUB).unwrap(),
                &pool,
            )
            .await?;
            // Geofence is the sub-area, point is inside the parent polygon
            // but well outside the sub-area polygon. Must be rejected.
            let user = user_with(vec![Role::EventManager], vec![subarea.id]);
            let err = check(&user, 8.10, 98.40, &pool).await.unwrap_err();
            assert!(err.to_string().contains("outside your geofence"));
            // Sanity: phuket is also created so we don't rely on subarea id
            // being 1 in test ordering.
            assert!(phuket.id != subarea.id);
            Ok::<(), crate::Error>(())
        })
    }
}
