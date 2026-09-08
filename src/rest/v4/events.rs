use crate::db;
use crate::db::main::event::schema::Event;
use crate::db::main::MainPool;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use geo::Contains;
use geo::LineString;
use geo::MultiPolygon;
use geo::Polygon;
use geojson::Geometry as GeoJsonGeometry;
use serde::Deserialize;
use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use time::macros::datetime;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Item {
    pub id: i64,
    pub area_id: Option<i64>,
    pub lat: f64,
    pub lon: f64,
    pub name: String,
    pub website: String,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
    pub cron_schedule: Option<String>,
}

impl From<Event> for Item {
    fn from(val: Event) -> Self {
        Item {
            id: val.id,
            area_id: val.area_id,
            lat: val.lat,
            lon: val.lon,
            name: val.name,
            website: val.website,
            starts_at: val.starts_at.unwrap_or(OffsetDateTime::UNIX_EPOCH),
            ends_at: val.ends_at,
            cron_schedule: val.cron_schedule,
        }
    }
}

#[derive(Deserialize)]
pub struct GetByAreaArgs {
    pub from: Option<String>,
    pub to: Option<String>,
}

#[get("")]
pub async fn get(pool: Data<MainPool>) -> RestResult<Vec<Item>> {
    let items = db::main::event::queries::select_all(&pool)
        .await
        .map_err(|_| RestApiError::database())?;
    let items: Vec<Event> = items
        .into_iter()
        .filter(|it| {
            it.deleted_at.is_none()
                && (it.starts_at.is_none() || it.starts_at > Some(OffsetDateTime::now_utc()))
        })
        .collect();
    Ok(Json(items.into_iter().map(|it| it.into()).collect()))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<MainPool>) -> RestResult<Item> {
    db::main::event::queries::select_by_id(id.into_inner(), &pool)
        .await
        .map(|it| Json(it.into()))
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })
}

#[get("{id}/events")]
pub async fn get_by_area(
    id: Path<String>,
    args: Query<GetByAreaArgs>,
    pool: Data<MainPool>,
) -> RestResult<Vec<Item>> {
    let id = id.into_inner();
    if id.len() > 128 {
        return Err(RestApiError::invalid_input("id too long"));
    }

    let from = match args.from.as_deref() {
        Some(s) => OffsetDateTime::parse(s, &Rfc3339)
            .map_err(|_| RestApiError::invalid_input("Invalid 'from' date"))?,
        None => OffsetDateTime::now_utc(),
    };
    let to = match args.to.as_deref() {
        Some(s) => OffsetDateTime::parse(s, &Rfc3339)
            .map_err(|_| RestApiError::invalid_input("Invalid 'to' date"))?,
        None => datetime!(2200-01-01 0:00 UTC),
    };

    let area = db::main::area::queries::select_by_id_or_alias(id, &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;

    let candidates = db::main::event::queries::select_by_bbox(
        area.bbox_west,
        area.bbox_south,
        area.bbox_east,
        area.bbox_north,
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;

    let geometries: Vec<GeoJsonGeometry> = area.geo_json_geometries().unwrap_or_default();
    let items: Vec<Item> = candidates
        .into_iter()
        .filter(|event| {
            if event.deleted_at.is_some() {
                return false;
            }
            if let Some(starts_at) = event.starts_at {
                if starts_at < from || starts_at > to {
                    return false;
                }
            }
            event_point_in_geometries(event.lon, event.lat, &geometries)
        })
        .map(|it| it.into())
        .collect();

    Ok(Json(items))
}

/// Cheap two-stage membership check: assume the candidate point is outside
/// the area unless at least one of the area's geometries reports it as
/// contained. The bbox pre-filter has already discarded obvious misses; this
/// loop only runs on the survivors.
fn event_point_in_geometries(lon: f64, lat: f64, geometries: &[GeoJsonGeometry]) -> bool {
    if geometries.is_empty() {
        return false;
    }
    let coord = geo::coord!(x: lon, y: lat);
    for geometry in geometries {
        match &geometry.value {
            geojson::GeometryValue::MultiPolygon { .. } => {
                let multi_poly: MultiPolygon = (&geometry.value).try_into().unwrap();
                if multi_poly.contains(&coord) {
                    return true;
                }
            }
            geojson::GeometryValue::Polygon { .. } => {
                let poly: Polygon = (&geometry.value).try_into().unwrap();
                if poly.contains(&coord) {
                    return true;
                }
            }
            geojson::GeometryValue::LineString { .. } => {
                let line_string: LineString = (&geometry.value).try_into().unwrap();
                if line_string.contains(&coord) {
                    return true;
                }
            }
            _ => continue,
        }
    }
    false
}

#[cfg(test)]
mod test {
    use crate::db::main::test::pool;
    use crate::{db, Result};
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use geojson::JsonObject;
    use time::macros::datetime;
    use time::OffsetDateTime;

    #[test]
    async fn get_empty_array() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_not_empty_array() -> Result<()> {
        let pool = pool();
        let event = db::main::event::queries::insert(
            Some(1),
            1.23,
            4.56,
            "name".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(event.id, res.first().unwrap()["id"].as_i64().unwrap());
        assert_eq!(1, res.first().unwrap()["area_id"].as_i64().unwrap());
        Ok(())
    }

    #[test]
    async fn get_excludes_deleted() -> Result<()> {
        let pool = pool();
        let event = db::main::event::queries::insert(
            None,
            1.23,
            4.56,
            "name".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        db::main::event::queries::set_deleted_at(event.id, Some(OffsetDateTime::now_utc()), &pool)
            .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_excludes_past_events() -> Result<()> {
        let pool = pool();
        db::main::event::queries::insert(
            None,
            1.23,
            4.56,
            "past_event".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2020-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        let future_event = db::main::event::queries::insert(
            None,
            7.89,
            10.11,
            "future_event".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(
            future_event.id,
            res.first().unwrap()["id"].as_i64().unwrap()
        );
        Ok(())
    }

    #[test]
    async fn get_by_id() -> Result<()> {
        let pool = pool();
        let event = db::main::event::queries::insert(
            Some(1),
            1.23,
            4.56,
            "name".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_id),
        )
        .await;
        let req = TestRequest::get().uri("/1").to_request();
        let res: JsonObject = test::call_and_read_body_json(&app, req).await;
        assert_eq!(event.id, res["id"].as_i64().unwrap());
        assert_eq!(1, res["area_id"].as_i64().unwrap());
        Ok(())
    }

    #[test]
    async fn get_by_id_not_found() -> Result<()> {
        let pool = pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_id),
        )
        .await;
        let req = TestRequest::get().uri("/999").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 404);
        Ok(())
    }

    fn phuket_area_tags() -> serde_json::Map<String, serde_json::Value> {
        let mut tags = crate::db::main::area::schema::Area::mock_tags();
        tags.insert(
            "geo_json".into(),
            serde_json::json!({
                "type": "Feature",
                "properties": {},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[
                        [98.21, 7.74],
                        [98.49, 7.74],
                        [98.35, 8.21],
                        [98.21, 7.74]
                    ]]
                }
            }),
        );
        tags
    }

    #[test]
    async fn get_by_area_returns_empty_when_no_events() -> Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(phuket_area_tags(), &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/events", area.id))
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_by_area_includes_event_inside_polygon() -> Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(phuket_area_tags(), &pool).await?;
        let inside = db::main::event::queries::insert(
            Some(area.id),
            7.97,
            98.33,
            "inside".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/events", area.id))
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(inside.id, res[0]["id"].as_i64().unwrap());
        Ok(())
    }

    #[test]
    async fn get_by_area_excludes_event_outside_bbox() -> Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(phuket_area_tags(), &pool).await?;
        db::main::event::queries::insert(
            Some(area.id),
            51.5,
            -0.1,
            "london".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/events", area.id))
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_by_area_excludes_event_in_bbox_but_outside_polygon() -> Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(phuket_area_tags(), &pool).await?;
        // Triangle's bbox is lat [7.74, 8.21] / lon [98.21, 98.49].
        // (98.25, 8.10) is inside the bbox but west of the slanted
        // hypotenuse, so it's outside the polygon.
        db::main::event::queries::insert(
            Some(area.id),
            8.10,
            98.25,
            "sea".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/events", area.id))
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(
            res.is_empty(),
            "events in the bbox but outside the polygon must be filtered out"
        );
        Ok(())
    }

    #[test]
    async fn get_by_area_excludes_past_events_by_default() -> Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(phuket_area_tags(), &pool).await?;
        db::main::event::queries::insert(
            Some(area.id),
            7.97,
            98.33,
            "past".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2020-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/events", area.id))
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_by_area_from_includes_past_events() -> Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(phuket_area_tags(), &pool).await?;
        let past = db::main::event::queries::insert(
            Some(area.id),
            7.97,
            98.33,
            "past".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2020-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        db::main::event::queries::insert(
            Some(area.id),
            7.97,
            98.33,
            "future".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/events?from=2019-01-01T00:00:00Z", area.id))
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, res.len());
        assert_eq!(past.id, res[0]["id"].as_i64().unwrap());
        Ok(())
    }

    #[test]
    async fn get_by_area_to_caps_upper_bound() -> Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(phuket_area_tags(), &pool).await?;
        db::main::event::queries::insert(
            Some(area.id),
            7.97,
            98.33,
            "in_window".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2024-06-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        db::main::event::queries::insert(
            Some(area.id),
            7.97,
            98.33,
            "too_late".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!(
                "/{}/events?from=2020-01-01T00:00:00Z&to=2025-01-01T00:00:00Z",
                area.id
            ))
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!("in_window", res[0]["name"].as_str().unwrap());
        Ok(())
    }

    #[test]
    async fn get_by_area_excludes_soft_deleted_events() -> Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(phuket_area_tags(), &pool).await?;
        let event = db::main::event::queries::insert(
            Some(area.id),
            7.97,
            98.33,
            "deleted".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        db::main::event::queries::set_deleted_at(event.id, Some(OffsetDateTime::now_utc()), &pool)
            .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/events", area.id))
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_by_area_returns_empty_when_area_has_no_geometry() -> Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(
            crate::db::main::area::schema::Area::mock_tags(),
            &pool,
        )
        .await?;
        db::main::event::queries::insert(
            Some(area.id),
            7.97,
            98.33,
            "orphan".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/events", area.id))
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(
            res.is_empty(),
            "areas without geo_json geometry cannot contain any event"
        );
        Ok(())
    }

    #[test]
    async fn get_by_area_resolves_alias() -> Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(phuket_area_tags(), &pool).await?;
        let event = db::main::event::queries::insert(
            Some(area.id),
            7.97,
            98.33,
            "alias_test".to_string(),
            "https://example.com".to_string(),
            Some(datetime!(2099-01-01 0:00 UTC)),
            None,
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get().uri("/alias/events").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(event.id, res[0]["id"].as_i64().unwrap());
        Ok(())
    }

    #[test]
    async fn get_by_area_404_for_unknown_area() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get().uri("/999/events").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 404);
        Ok(())
    }

    #[test]
    async fn get_by_area_400_for_invalid_from() -> Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(phuket_area_tags(), &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/events?from=not-a-date", area.id))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 400);
        Ok(())
    }

    #[test]
    async fn get_by_area_400_for_invalid_to() -> Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(phuket_area_tags(), &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_area),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/events?to=not-a-date", area.id))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 400);
        Ok(())
    }
}
