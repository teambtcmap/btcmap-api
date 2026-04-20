use crate::db;
use crate::db::main::MainPool;
use crate::rest::auth::Auth;
use crate::rest::error::RestResult as Res;
use crate::rest::error::{RestApiError, RestApiErrorCode};
use crate::rest::v4::top_editors::{
    extract_tip_url, far_future, parse_date, validate_limit, TopEditor, EXCLUDED_USER_IDS,
};
use crate::service;
use crate::Error;
use actix_web::{delete, get, post, put, web::Data, web::Json, web::Path, web::Query};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct SearchArgs {
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub r#type: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AreaSearchResult {
    pub id: i64,
    pub name: String,
    pub r#type: String,
    pub url_alias: String,
    pub icon: Option<String>,
    pub website_url: String,
}

#[get("")]
pub async fn get(args: Query<SearchArgs>, pool: Data<MainPool>) -> Res<Vec<AreaSearchResult>> {
    let type_filter = args.r#type.clone();

    let areas = if let (Some(lat), Some(lon)) = (args.lat, args.lon) {
        if !(-90.0..=90.0).contains(&lat) {
            return Err(RestApiError::new(
                RestApiErrorCode::InvalidInput,
                "Latitude must be between -90 and 90",
            ));
        }

        if !(-180.0..=180.0).contains(&lon) {
            return Err(RestApiError::new(
                RestApiErrorCode::InvalidInput,
                "Longitude must be between -180 and 180",
            ));
        }

        service::area::find_areas_by_lat_lon(lat, lon, &pool)
            .await
            .map_err(|_| RestApiError::database())?
    } else {
        db::main::area::queries::select(None, false, None, &pool)
            .await
            .map_err(|_| RestApiError::database())?
    };

    let results: Vec<AreaSearchResult> = areas
        .into_iter()
        .filter(|area| {
            if let Some(ref filter_type) = type_filter {
                let area_type = area.tags.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if area_type != filter_type {
                    return false;
                }
            }
            true
        })
        .map(|area| {
            let r#type = area.tags.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let singular_type = if let Some(stripped) = r#type.strip_suffix("ies") {
                format!("{}y", stripped)
            } else if let Some(stripped) = r#type.strip_suffix('s') {
                stripped.to_string()
            } else {
                r#type.to_string()
            };
            let url_alias = area.alias();
            AreaSearchResult {
                id: area.id,
                name: area.name(),
                r#type: r#type.to_string(),
                url_alias: url_alias.clone(),
                icon: area
                    .tags
                    .get("icon:square")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                website_url: format!("https://btcmap.org/{}/{}", singular_type, url_alias),
            }
        })
        .collect();

    Ok(Json(results))
}

#[derive(Serialize, Deserialize)]
pub struct GetByIdRes {
    pub id: i64,
    pub name: String,
    pub r#type: String,
    pub url_alias: String,
    pub icon: Option<String>,
    pub icon_wide: Option<String>,
    pub website_url: String,
    pub description: String,
}

#[get("{id}")]
pub async fn get_by_id(id: Path<String>, pool: Data<MainPool>) -> Res<GetByIdRes> {
    if id.len() > 128 {
        return Err(RestApiError::invalid_input("id too long"));
    }
    let area = db::main::area::queries::select_by_id_or_alias(id.into_inner(), &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;
    let r#type = area.tags.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let singular_type = if let Some(stripped) = r#type.strip_suffix("ies") {
        format!("{}y", stripped)
    } else if let Some(stripped) = r#type.strip_suffix('s') {
        stripped.to_string()
    } else {
        r#type.to_string()
    };
    let url_alias = area.alias();
    let description = area
        .tags
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    Ok(Json(GetByIdRes {
        id: area.id,
        name: area.name(),
        r#type: r#type.to_string(),
        url_alias: url_alias.clone(),
        icon: area
            .tags
            .get("icon:square")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        icon_wide: area
            .tags
            .get("icon:wide")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        website_url: format!("https://btcmap.org/{}/{}", singular_type, url_alias),
        description,
    }))
}

#[get("/saved")]
pub async fn get_saved(auth: Auth, pool: Data<MainPool>) -> Res<Vec<AreaSearchResult>> {
    let user = auth.user.ok_or(RestApiError::unauthorized())?;
    let areas = db::main::area::queries::select_by_ids(&user.saved_areas, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    let results: Vec<AreaSearchResult> = areas
        .into_iter()
        .map(|area| {
            let r#type = area.tags.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let singular_type = if let Some(stripped) = r#type.strip_suffix("ies") {
                format!("{}y", stripped)
            } else if let Some(stripped) = r#type.strip_suffix('s') {
                stripped.to_string()
            } else {
                r#type.to_string()
            };
            let url_alias = area.alias();
            AreaSearchResult {
                id: area.id,
                name: area.name(),
                r#type: r#type.to_string(),
                url_alias: url_alias.clone(),
                icon: area
                    .tags
                    .get("icon:square")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                website_url: format!("https://btcmap.org/{}/{}", singular_type, url_alias),
            }
        })
        .collect();
    Ok(Json(results))
}

#[put("/saved")]
pub async fn put_saved(auth: Auth, args: Json<Vec<i64>>, pool: Data<MainPool>) -> Res<Vec<i64>> {
    let user = auth.user.ok_or(RestApiError::unauthorized())?;
    db::main::user::queries::set_saved_areas(user.id, &args, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(actix_web::web::Json(args.into_inner()))
}

#[post("/saved")]
pub async fn post_saved(auth: Auth, args: Json<i64>, pool: Data<MainPool>) -> Res<Vec<i64>> {
    let user = auth.user.ok_or(RestApiError::unauthorized())?;
    let mut saved_areas = user.saved_areas.clone();
    if !saved_areas.contains(&args) {
        saved_areas.push(args.into_inner());
    }
    db::main::user::queries::set_saved_areas(user.id, &saved_areas, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(actix_web::web::Json(saved_areas))
}

#[delete("/saved/{id}")]
pub async fn delete_saved(auth: Auth, path: Path<i64>, pool: Data<MainPool>) -> Res<Vec<i64>> {
    let user = auth.user.ok_or(RestApiError::unauthorized())?;
    let id = path.into_inner();
    let mut saved_areas = user.saved_areas.clone();
    saved_areas.retain(|x| *x != id);
    db::main::user::queries::set_saved_areas(user.id, &saved_areas, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(actix_web::web::Json(saved_areas))
}

#[derive(Deserialize)]
pub struct GetTopEditorsForAreaArgs {
    period_start: Option<String>,
    period_end: Option<String>,
    limit: Option<i64>,
}

/// Return the most active editors whose changes fell on places inside the given
/// area. Mirrors the global `/v4/top-editors` response shape and bot-id
/// blocklist; the only difference is the area scope and the date-range params
/// being optional (default to an open range).
#[get("{id}/top-editors")]
pub async fn get_by_id_top_editors(
    id: Path<String>,
    args: Query<GetTopEditorsForAreaArgs>,
    pool: Data<MainPool>,
) -> Res<Vec<TopEditor>> {
    if id.len() > 128 {
        return Err(RestApiError::invalid_input("id too long"));
    }
    let area = db::main::area::queries::select_by_id_or_alias(id.into_inner(), &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;

    let period_start = match args.period_start.as_deref() {
        Some(s) => parse_date(s, true)?,
        None => OffsetDateTime::UNIX_EPOCH,
    };
    let period_end = match args.period_end.as_deref() {
        Some(s) => parse_date(s, false)?,
        None => far_future(),
    };
    let limit = validate_limit(args.limit)?;

    let editors = db::main::osm_user::queries::select_most_active_for_area(
        area.id,
        period_start,
        period_end,
        limit,
        EXCLUDED_USER_IDS,
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;

    let editors: Vec<TopEditor> = editors
        .into_iter()
        .map(|e| TopEditor {
            id: e.id,
            name: e.name,
            avatar_url: e.image_url,
            total_edits: e.edits,
            places_created: e.created,
            places_updated: e.updated,
            places_deleted: e.deleted,
            tip_url: extract_tip_url(&e.description),
        })
        .collect();

    Ok(Json(editors))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::main::area::schema::Area;
    use crate::db::main::test::pool;
    use crate::{db, Result};
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use serde_json::json;

    #[test]
    async fn search_invalid_lat_returns_400() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?lat=91&lon=0").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 400);
        Ok(())
    }

    #[test]
    async fn search_invalid_lon_returns_400() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?lat=0&lon=181").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 400);
        Ok(())
    }

    #[test]
    async fn search_returns_results() -> Result<()> {
        let pool = pool();

        let mut tags = Area::mock_tags();
        tags.insert("name".into(), json!("Phuket"));
        tags.insert("type".into(), json!("country"));
        tags.insert(
            "geo_json".into(),
            json!(
                {
                    "type": "FeatureCollection",
                    "features": [
                      {
                        "type": "Feature",
                        "properties": {},
                        "geometry": {
                          "coordinates": [
                            [
                              [98.2181205776469, 8.20412838698085],
                              [98.2181205776469, 7.74024270965898],
                              [98.4806081271079, 7.74024270965898],
                              [98.4806081271079, 8.20412838698085],
                              [98.2181205776469, 8.20412838698085]
                            ]
                          ],
                          "type": "Polygon"
                        }
                      }
                    ]
                  }
            ),
        );
        db::main::area::queries::insert(tags, &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;

        let req = TestRequest::get().uri("/?lat=7.9&lon=98.3").to_request();
        let res: Vec<AreaSearchResult> = test::call_and_read_body_json(&app, req).await;

        assert!(!res.is_empty());
        assert_eq!(res[0].name, "Phuket");
        Ok(())
    }

    #[test]
    async fn get_by_id_returns_area() -> Result<()> {
        let pool = pool();
        let mut tags = Area::mock_tags();
        tags.insert("name".into(), json!("Phuket"));
        tags.insert("type".into(), json!("country"));
        tags.insert(
            "description".into(),
            json!("A beautiful island in Thailand"),
        );
        tags.insert(
            "geo_json".into(),
            json!(
                {
                    "type": "FeatureCollection",
                    "features": [
                      {
                        "type": "Feature",
                        "properties": {},
                        "geometry": {
                          "coordinates": [
                            [
                              [98.2181205776469, 8.20412838698085],
                              [98.2181205776469, 7.74024270965898],
                              [98.4806081271079, 7.74024270965898],
                              [98.4806081271079, 8.20412838698085],
                              [98.2181205776469, 8.20412838698085]
                            ]
                          ],
                          "type": "Polygon"
                        }
                      }
                    ]
                  }
            ),
        );
        let area = db::main::area::queries::insert(tags, &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/areas").service(super::get_by_id)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/areas/{}", area.id))
            .to_request();
        let res: GetByIdRes = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.name, "Phuket");
        assert_eq!(res.description, "A beautiful island in Thailand");
        Ok(())
    }

    #[test]
    async fn top_editors_for_unknown_area_returns_404() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(super::get_by_id_top_editors),
        )
        .await;
        let req = TestRequest::get().uri("/9999/top-editors").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 404);
        Ok(())
    }

    #[test]
    async fn top_editors_for_area_scopes_to_area_elements() -> Result<()> {
        use crate::service::osm::EditingApiUser;
        use crate::service::overpass::OverpassElement;

        let pool = pool();
        let area = db::main::area::queries::insert(Area::mock_tags(), &pool).await?;
        let user = db::main::osm_user::queries::insert(1, EditingApiUser::mock(), &pool).await?;
        let other_user =
            db::main::osm_user::queries::insert(2, EditingApiUser::mock(), &pool).await?;

        let element_in_area =
            db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let element_outside =
            db::main::element::queries::insert(OverpassElement::mock(2), &pool).await?;

        db::main::area_element::queries::insert(area.id, element_in_area.id, &pool).await?;

        // 2 in-area edits by `user`
        db::main::element_event::queries::insert(user.id, element_in_area.id, "update", &pool)
            .await?;
        db::main::element_event::queries::insert(user.id, element_in_area.id, "update", &pool)
            .await?;
        // 1 in-area edit by `other_user`
        db::main::element_event::queries::insert(
            other_user.id,
            element_in_area.id,
            "create",
            &pool,
        )
        .await?;
        // 1 out-of-area edit by `user` — must be excluded.
        db::main::element_event::queries::insert(user.id, element_outside.id, "update", &pool)
            .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_id_top_editors),
        )
        .await;

        let req = TestRequest::get()
            .uri(&format!("/{}/top-editors", area.id))
            .to_request();
        let res: Vec<TopEditor> = test::call_and_read_body_json(&app, req).await;

        assert_eq!(2, res.len());
        // user (id=1) leads with 2 in-area edits; other_user with 1.
        assert_eq!(user.id, res[0].id);
        assert_eq!(2, res[0].total_edits);
        assert_eq!(2, res[0].places_updated);
        assert_eq!(0, res[0].places_created);
        assert_eq!(other_user.id, res[1].id);
        assert_eq!(1, res[1].total_edits);
        assert_eq!(1, res[1].places_created);
        Ok(())
    }

    #[test]
    async fn top_editors_for_area_respects_limit() -> Result<()> {
        use crate::service::osm::EditingApiUser;
        use crate::service::overpass::OverpassElement;

        let pool = pool();
        let area = db::main::area::queries::insert(Area::mock_tags(), &pool).await?;
        let element = db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        db::main::area_element::queries::insert(area.id, element.id, &pool).await?;

        for uid in 1..=3 {
            let u = db::main::osm_user::queries::insert(uid, EditingApiUser::mock(), &pool).await?;
            db::main::element_event::queries::insert(u.id, element.id, "update", &pool).await?;
        }

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_id_top_editors),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/top-editors?limit=2", area.id))
            .to_request();
        let res: Vec<TopEditor> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, res.len());
        Ok(())
    }
}
