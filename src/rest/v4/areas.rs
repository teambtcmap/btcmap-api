use crate::db;
use crate::db::image::ImagePool;
use crate::db::main::MainPool;
use crate::rest::auth::Auth;
use crate::rest::error::RestResult as Res;
use crate::rest::error::{RestApiError, RestApiErrorCode};
use crate::rest::v4::top_editors::{
    extract_tip_url, far_future, parse_date, validate_limit, TopEditor, EXCLUDED_USER_IDS,
};
use crate::service;
use crate::Error;
use actix_web::{
    delete, get, post, put, web::Data, web::Json, web::Path, web::Query, HttpResponse,
};
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

#[derive(Deserialize)]
pub struct GetImageArgs {
    pub r#type: String,
    pub w: Option<u32>,
    pub h: Option<u32>,
}

#[get("{id}/image")]
pub async fn get_by_id_image(
    id: Path<String>,
    args: Query<GetImageArgs>,
    pool: Data<MainPool>,
    image_pool: Data<ImagePool>,
) -> Result<HttpResponse, RestApiError> {
    if id.len() > 128 {
        return Err(RestApiError::invalid_input("id too long"));
    }
    if let Some(0) = args.w {
        return Err(RestApiError::invalid_input("w must be greater than 0"));
    }
    if let Some(0) = args.h {
        return Err(RestApiError::invalid_input("h must be greater than 0"));
    }
    let area = db::main::area::queries::select_by_id_or_alias(id.into_inner(), &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;

    let image =
        db::image::area::queries::select_by_area_id_and_type(area.id, &args.r#type, &image_pool)
            .await
            .map_err(|_| RestApiError::database())?
            .ok_or(RestApiError::not_found())?;

    let bytes = image.image_data;
    let resize_requested = args.w.is_some() || args.h.is_some();

    if !resize_requested || looks_like_svg(&bytes) {
        let content_type =
            content_type_for(&bytes).unwrap_or_else(|| "application/octet-stream".to_string());
        return Ok(HttpResponse::Ok().content_type(content_type).body(bytes));
    }

    let w_req = args.w;
    let h_req = args.h;

    let resized = actix_web::web::block(move || -> Result<(Vec<u8>, String), RestApiError> {
        let format = match image::guess_format(&bytes) {
            Ok(f) => f,
            Err(_) => {
                let ct = content_type_for(&bytes)
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                return Ok((bytes, ct));
            }
        };

        let content_type: &'static str = match format {
            image::ImageFormat::Png => "image/png",
            image::ImageFormat::Jpeg => "image/jpeg",
            image::ImageFormat::WebP => "image/webp",
            _ => {
                let ct = content_type_for(&bytes)
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                return Ok((bytes, ct));
            }
        };

        let img = image::load_from_memory(&bytes).map_err(|_| RestApiError::database())?;
        let (src_w, src_h) = (img.width(), img.height());
        let (target_w, target_h) = fit_dimensions(src_w, src_h, w_req, h_req);

        if target_w == src_w && target_h == src_h {
            return Ok((bytes, content_type.to_string()));
        }

        let resized_img = img.resize(target_w, target_h, image::imageops::FilterType::Triangle);
        let mut out: Vec<u8> = Vec::new();
        match format {
            image::ImageFormat::Png => {
                let encoder = image::codecs::png::PngEncoder::new(&mut out);
                resized_img
                    .write_with_encoder(encoder)
                    .map_err(|_| RestApiError::database())?;
            }
            image::ImageFormat::Jpeg => {
                let encoder = image::codecs::jpeg::JpegEncoder::new(&mut out);
                resized_img
                    .write_with_encoder(encoder)
                    .map_err(|_| RestApiError::database())?;
            }
            image::ImageFormat::WebP => {
                let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut out);
                resized_img
                    .write_with_encoder(encoder)
                    .map_err(|_| RestApiError::database())?;
            }
            _ => unreachable!(),
        }
        Ok((out, content_type.to_string()))
    })
    .await
    .map_err(|_| RestApiError::database())??;

    Ok(HttpResponse::Ok().content_type(resized.1).body(resized.0))
}

/// Pick target dimensions that fit the source into the requested box without
/// upsizing. If only one bound is provided, the other is derived from the
/// source aspect ratio. When the source already fits, it is returned as-is.
fn fit_dimensions(src_w: u32, src_h: u32, w: Option<u32>, h: Option<u32>) -> (u32, u32) {
    match (w, h) {
        (None, None) => (src_w, src_h),
        (Some(mw), None) => {
            if mw >= src_w {
                (src_w, src_h)
            } else {
                (mw, src_h * mw / src_w)
            }
        }
        (None, Some(mh)) => {
            if mh >= src_h {
                (src_w, src_h)
            } else {
                (src_w * mh / src_h, mh)
            }
        }
        (Some(mw), Some(mh)) => {
            if src_w <= mw && src_h <= mh {
                return (src_w, src_h);
            }
            let ratio = (mw as f64 / src_w as f64).min(mh as f64 / src_h as f64);
            let nw = ((src_w as f64) * ratio).round() as u32;
            let nh = ((src_h as f64) * ratio).round() as u32;
            (nw.max(1), nh.max(1))
        }
    }
}

fn content_type_for(bytes: &[u8]) -> Option<String> {
    if looks_like_svg(bytes) {
        return Some("image/svg+xml".to_string());
    }
    let format = image::guess_format(bytes).ok()?;
    Some(
        match format {
            image::ImageFormat::Png => "image/png",
            image::ImageFormat::Jpeg => "image/jpeg",
            image::ImageFormat::WebP => "image/webp",
            image::ImageFormat::Bmp => "image/bmp",
            _ => "application/octet-stream",
        }
        .to_string(),
    )
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(512)];
    let Ok(head) = std::str::from_utf8(head) else {
        return false;
    };
    let trimmed = head.trim_start();
    trimmed.starts_with("<?xml") || trimmed.starts_with("<svg")
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

    #[test]
    async fn image_returns_404_for_unknown_area() -> Result<()> {
        let image_pool = crate::db::image::test::pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .app_data(Data::new(image_pool))
                .service(scope("/areas").service(super::get_by_id_image)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/areas/9999/image?type=square")
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 404);
        Ok(())
    }

    #[test]
    async fn image_returns_404_when_no_cached_image() -> Result<()> {
        let main_pool = pool();
        let area = db::main::area::queries::insert(Area::mock_tags(), &main_pool).await?;
        let image_pool = crate::db::image::test::pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool))
                .service(scope("/areas").service(super::get_by_id_image)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/areas/{}/image?type=square", area.id))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 404);
        Ok(())
    }

    #[test]
    async fn image_returns_cached_png_with_correct_content_type() -> Result<()> {
        let main_pool = pool();
        let area = db::main::area::queries::insert(Area::mock_tags(), &main_pool).await?;
        let image_pool = crate::db::image::test::pool();
        let png_bytes: Vec<u8> = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52,
        ];
        db::image::area::queries::insert(
            area.id,
            "square",
            png_bytes.clone(),
            1,
            1,
            png_bytes.len() as i64,
            &image_pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool))
                .service(scope("/areas").service(super::get_by_id_image)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/areas/{}/image?type=square", area.id))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
        assert_eq!(
            res.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("image/png")
        );
        let body = test::read_body(res).await.to_vec();
        assert_eq!(body, png_bytes);
        Ok(())
    }

    #[test]
    async fn image_returns_cached_svg_with_correct_content_type() -> Result<()> {
        let main_pool = pool();
        let area = db::main::area::queries::insert(Area::mock_tags(), &main_pool).await?;
        let image_pool = crate::db::image::test::pool();
        let svg_bytes = br#"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"></svg>"#;
        db::image::area::queries::insert(
            area.id,
            "square",
            svg_bytes.to_vec(),
            32,
            32,
            svg_bytes.len() as i64,
            &image_pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool))
                .service(scope("/areas").service(super::get_by_id_image)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/areas/{}/image?type=square", area.id))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
        assert_eq!(
            res.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("image/svg+xml")
        );
        Ok(())
    }

    #[test]
    async fn image_selects_by_requested_type() -> Result<()> {
        let main_pool = pool();
        let area = db::main::area::queries::insert(Area::mock_tags(), &main_pool).await?;
        let image_pool = crate::db::image::test::pool();
        let square_bytes: Vec<u8> = vec![1, 2, 3, 4];
        let wide_bytes: Vec<u8> = vec![5, 6, 7, 8, 9, 10];
        db::image::area::queries::insert(
            area.id,
            "square",
            square_bytes.clone(),
            32,
            32,
            square_bytes.len() as i64,
            &image_pool,
        )
        .await?;
        db::image::area::queries::insert(
            area.id,
            "wide",
            wide_bytes.clone(),
            256,
            64,
            wide_bytes.len() as i64,
            &image_pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool))
                .service(scope("/areas").service(super::get_by_id_image)),
        )
        .await;

        let req = TestRequest::get()
            .uri(&format!("/areas/{}/image?type=square", area.id))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
        assert_eq!(test::read_body(res).await.to_vec(), square_bytes);

        let req = TestRequest::get()
            .uri(&format!("/areas/{}/image?type=wide", area.id))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
        assert_eq!(test::read_body(res).await.to_vec(), wide_bytes);

        Ok(())
    }

    #[::core::prelude::v1::test]
    fn content_type_for_detects_png() {
        let png_bytes: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(
            Some("image/png".to_string()),
            super::content_type_for(&png_bytes)
        );
    }

    #[::core::prelude::v1::test]
    fn content_type_for_detects_jpeg() {
        let jpeg_bytes: Vec<u8> = vec![0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(
            Some("image/jpeg".to_string()),
            super::content_type_for(&jpeg_bytes)
        );
    }

    #[::core::prelude::v1::test]
    fn content_type_for_detects_svg() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        assert_eq!(
            Some("image/svg+xml".to_string()),
            super::content_type_for(svg)
        );
    }

    #[::core::prelude::v1::test]
    fn content_type_for_returns_none_for_unknown() {
        let bytes = b"definitely not an image";
        assert_eq!(None, super::content_type_for(bytes));
    }

    fn encode_png(width: u32, height: u32) -> Vec<u8> {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(width, height, Rgb([255, 0, 0]));
        let mut out: Vec<u8> = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut out);
        img.write_with_encoder(encoder).unwrap();
        out
    }

    async fn make_image(
        main_pool: &crate::db::main::MainPool,
        image_pool: &crate::db::image::ImagePool,
        width: i64,
        height: i64,
        bytes: Vec<u8>,
    ) -> crate::Result<crate::db::main::area::schema::Area> {
        let area = db::main::area::queries::insert(Area::mock_tags(), main_pool).await?;
        db::image::area::queries::insert(area.id, "square", bytes, width, height, 0, image_pool)
            .await?;
        Ok(area)
    }

    #[test]
    async fn image_resize_returns_smaller_png_when_requested_lower() -> Result<()> {
        let main_pool = pool();
        let image_pool = crate::db::image::test::pool();
        let src_bytes = encode_png(100, 100);
        let area = make_image(&main_pool, &image_pool, 100, 100, src_bytes.clone()).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool))
                .service(scope("/areas").service(super::get_by_id_image)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/areas/{}/image?type=square&w=50&h=50", area.id))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
        assert_eq!(
            res.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("image/png")
        );
        let body = test::read_body(res).await.to_vec();
        assert_ne!(body, src_bytes, "resized image should differ from source");
        let decoded = image::load_from_memory(&body).unwrap();
        assert_eq!(decoded.width(), 50);
        assert_eq!(decoded.height(), 50);
        Ok(())
    }

    #[test]
    async fn image_resize_returns_original_when_requested_larger() -> Result<()> {
        let main_pool = pool();
        let image_pool = crate::db::image::test::pool();
        let src_bytes = encode_png(100, 100);
        let area = make_image(&main_pool, &image_pool, 100, 100, src_bytes.clone()).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool))
                .service(scope("/areas").service(super::get_by_id_image)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/areas/{}/image?type=square&w=400&h=400", area.id))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
        let body = test::read_body(res).await.to_vec();
        assert_eq!(body, src_bytes, "must not upsize — return original bytes");
        let decoded = image::load_from_memory(&body).unwrap();
        assert_eq!(decoded.width(), 100);
        assert_eq!(decoded.height(), 100);
        Ok(())
    }

    #[test]
    async fn image_resize_keeps_aspect_ratio_with_only_width() -> Result<()> {
        let main_pool = pool();
        let image_pool = crate::db::image::test::pool();
        let src_bytes = encode_png(200, 100);
        let area = make_image(&main_pool, &image_pool, 200, 100, src_bytes).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool))
                .service(scope("/areas").service(super::get_by_id_image)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/areas/{}/image?type=square&w=100", area.id))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
        let body = test::read_body(res).await.to_vec();
        let decoded = image::load_from_memory(&body).unwrap();
        assert_eq!(decoded.width(), 100);
        assert_eq!(decoded.height(), 50);
        Ok(())
    }

    #[test]
    async fn image_resize_fits_into_box_when_both_dims_provided() -> Result<()> {
        let main_pool = pool();
        let image_pool = crate::db::image::test::pool();
        let src_bytes = encode_png(200, 100);
        let area = make_image(&main_pool, &image_pool, 200, 100, src_bytes).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool))
                .service(scope("/areas").service(super::get_by_id_image)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/areas/{}/image?type=square&w=100&h=100", area.id))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
        let body = test::read_body(res).await.to_vec();
        let decoded = image::load_from_memory(&body).unwrap();
        assert!(
            decoded.width() <= 100 && decoded.height() <= 100,
            "result must fit within the requested box"
        );
        assert_eq!(decoded.width(), 100);
        assert_eq!(decoded.height(), 50);
        Ok(())
    }

    #[test]
    async fn image_resize_skips_svg() -> Result<()> {
        let main_pool = pool();
        let image_pool = crate::db::image::test::pool();
        let svg_bytes =
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200"></svg>"#.to_vec();
        let area = make_image(&main_pool, &image_pool, 200, 200, svg_bytes.clone()).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(main_pool))
                .app_data(Data::new(image_pool))
                .service(scope("/areas").service(super::get_by_id_image)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/areas/{}/image?type=square&w=50&h=50", area.id))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
        assert_eq!(
            res.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("image/svg+xml")
        );
        assert_eq!(test::read_body(res).await.to_vec(), svg_bytes);
        Ok(())
    }

    #[test]
    async fn image_resize_rejects_zero_w() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/areas").service(super::get_by_id_image)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/areas/1/image?type=square&w=0")
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 400);
        Ok(())
    }

    #[test]
    async fn image_resize_rejects_zero_h() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .app_data(Data::new(crate::db::image::test::pool()))
                .service(scope("/areas").service(super::get_by_id_image)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/areas/1/image?type=square&h=0")
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 400);
        Ok(())
    }

    #[::core::prelude::v1::test]
    fn fit_dimensions_no_constraints_returns_source() {
        assert_eq!((100, 200), super::fit_dimensions(100, 200, None, None));
    }

    #[::core::prelude::v1::test]
    fn fit_dimensions_only_w_scales_height() {
        assert_eq!((50, 100), super::fit_dimensions(100, 200, Some(50), None));
    }

    #[::core::prelude::v1::test]
    fn fit_dimensions_only_h_scales_width() {
        assert_eq!((50, 100), super::fit_dimensions(100, 200, None, Some(100)));
    }

    #[::core::prelude::v1::test]
    fn fit_dimensions_w_not_upsizing_returns_source() {
        assert_eq!((100, 200), super::fit_dimensions(100, 200, Some(200), None));
    }

    #[::core::prelude::v1::test]
    fn fit_dimensions_box_fit_uses_smaller_ratio() {
        // source 200x100 fitting into 100x100 -> width-bound: ratio 0.5 -> 100x50
        assert_eq!(
            (100, 50),
            super::fit_dimensions(200, 100, Some(100), Some(100))
        );
    }

    #[::core::prelude::v1::test]
    fn fit_dimensions_box_already_fits_returns_source() {
        assert_eq!(
            (50, 50),
            super::fit_dimensions(50, 50, Some(200), Some(200))
        );
    }
}
