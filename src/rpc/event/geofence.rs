use crate::{
    db::{self, main::area::schema::Area, main::user::schema::Role},
    Result,
};
use deadpool_sqlite::Pool;
use geo::Contains;
use geo::LineString;
use geo::MultiPolygon;
use geo::Polygon;
use std::collections::HashSet;

/// Verify that the (area_id, lat, lon) the caller wants to operate on is
/// inside the caller's geofence, when the caller is an event manager with a
/// non-empty geofence.
///
/// Returns `Ok(true)` when no check is needed (user is not an event manager,
/// or has an empty geofence) or when the point falls inside the fence.
///
/// Returns an error when the user is an event manager, has a non-empty
/// geofence, and the point falls outside every fence area.
pub(crate) async fn check(
    user: &crate::db::main::user::schema::User,
    area_id: Option<i64>,
    lat: f64,
    lon: f64,
    pool: &Pool,
) -> Result<bool> {
    if !user.roles.contains(&Role::EventManager) {
        return Ok(true);
    }
    if user.geofence.is_empty() {
        return Ok(true);
    }

    let allowed_ids: HashSet<i64> = user.geofence.iter().copied().collect();

    if let Some(id) = area_id {
        if allowed_ids.contains(&id) {
            return Ok(true);
        }
        return Err(format!(
            "Area {id} is outside your geofence (allowed: {:?})",
            user.geofence
        )
        .into());
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
/// then run the geofence check against its stored (area_id, lat, lon).
pub(crate) async fn check_existing(
    user: &crate::db::main::user::schema::User,
    event_id: i64,
    pool: &Pool,
) -> Result<()> {
    let event = db::main::event::queries::select_by_id(event_id, pool).await?;
    check(user, event.area_id, event.lat, event.lon, pool)
        .await
        .map(|_| ())
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
    fn admin_user_is_unconstrained() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let admin = user_with(vec![Role::Admin], vec![]);
            assert!(check(&admin, None, 0.0, 0.0, &pool).await?);
            assert!(check(&admin, Some(999), 0.0, 0.0, &pool).await?);
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
            assert!(check(&user, None, 0.0, 0.0, &pool).await?);
            assert!(check(&user, Some(123), 0.0, 0.0, &pool).await?);
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn event_manager_with_geofence_allows_area_id_inside() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let phuket =
                insert_area("phuket", serde_json::from_str(PHUKET).unwrap(), &pool).await?;
            let user = user_with(vec![Role::EventManager], vec![phuket.id]);
            assert!(check(&user, Some(phuket.id), 0.0, 0.0, &pool).await?);
            Ok::<(), crate::Error>(())
        })
    }

    #[test]
    fn event_manager_with_geofence_rejects_area_id_outside() -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async {
            let pool = pool();
            let phuket =
                insert_area("phuket", serde_json::from_str(PHUKET).unwrap(), &pool).await?;
            let user = user_with(vec![Role::EventManager], vec![phuket.id]);
            let err = check(&user, Some(999), 0.0, 0.0, &pool).await.unwrap_err();
            assert!(err.to_string().contains("outside your geofence"));
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
            assert!(check(&user, None, 7.98, 98.33, &pool).await?);
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
            let err = check(&user, None, 51.5, -0.1, &pool).await.unwrap_err();
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
            assert!(check(&user, None, 7.98, 98.33, &pool).await?);
            // London point passes
            assert!(check(&user, None, 51.5, -0.1, &pool).await?);
            // In-between point fails
            let err = check(&user, None, 40.0, 50.0, &pool).await.unwrap_err();
            assert!(err.to_string().contains("outside your geofence"));
            Ok::<(), crate::Error>(())
        })
    }
}
