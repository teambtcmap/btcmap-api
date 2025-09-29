use crate::db;
use crate::db::element::schema::Element;
use crate::db::element_comment::schema::ElementComment;
use crate::db::place_submission::schema::PlaceSubmission;
use crate::log::RequestExtension;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult as Res;
use crate::service;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::HttpMessage;
use actix_web::HttpRequest;
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use serde::Deserialize;
use serde::Serialize;
use std::i64;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetListArgs {
    fields: Option<String>,
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    updated_since: Option<OffsetDateTime>,
    limit: Option<i64>,
    include_deleted: Option<bool>,
    include_pending: Option<bool>,
    prevent_pending_id_clash: Option<bool>,
}

#[derive(Deserialize)]
pub struct GetSingleArgs {
    fields: Option<String>,
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetListArgs>,
    pool: Data<Pool>,
) -> Res<Vec<JsonObject>> {
    let fields: Vec<&str> = args.fields.as_deref().unwrap_or("").split(',').collect();
    let updated_since = args.updated_since.unwrap_or(OffsetDateTime::UNIX_EPOCH);
    let include_deleted = args.include_deleted.unwrap_or(false) || fields.contains(&"deleted_at");
    let include_pending = args.include_pending.unwrap_or(false);
    let prevent_pending_id_clash = args.prevent_pending_id_clash.unwrap_or(true);

    let elements = db::element::queries::select_updated_since(
        updated_since,
        args.limit,
        include_deleted,
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;

    let mut submissions: Vec<PlaceSubmission> = vec![];

    if include_pending {
        submissions.append(
            &mut db::place_submission::queries::select_updated_since(
                updated_since,
                args.limit,
                include_deleted,
                &pool,
            )
            .await
            .map_err(|_| RestApiError::database())?,
        );
    }

    req.extensions_mut()
        .insert(RequestExtension::new(elements.len() + submissions.len()));

    if submissions.is_empty() {
        let elements = elements
            .into_iter()
            .map(|it| service::element::generate_tags(&it, &fields))
            .collect();

        Ok(Json(elements))
    } else {
        let mut items: Vec<GetItem> = vec![];
        let mut elements: Vec<GetItem> = elements
            .into_iter()
            .map(|it| GetItem::Element(it))
            .collect();
        let mut submissions: Vec<GetItem> = submissions
            .into_iter()
            .map(|it| GetItem::PlaceSubmission(it))
            .collect();
        items.append(&mut elements);
        items.append(&mut submissions);

        items.sort_by(|a, b| a.updated_at().cmp(&b.updated_at()));

        Ok(Json(
            items
                .into_iter()
                .map(|it| it.to_json(&fields, prevent_pending_id_clash))
                .take(args.limit.unwrap_or(i64::MAX) as usize)
                .collect(),
        ))
    }
}

pub enum GetItem {
    Element(Element),
    PlaceSubmission(PlaceSubmission),
}

impl GetItem {
    fn updated_at(&self) -> OffsetDateTime {
        match self {
            GetItem::Element(element) => element.updated_at,
            GetItem::PlaceSubmission(place_submission) => place_submission.updated_at,
        }
    }

    fn to_json(&self, fields: &Vec<&str>, prevent_pending_id_clash: bool) -> JsonObject {
        match self {
            GetItem::Element(element) => service::element::generate_tags(element, fields),
            GetItem::PlaceSubmission(place_submission) => {
                service::element::generate_submission_tags(
                    place_submission,
                    fields,
                    prevent_pending_id_clash,
                )
            }
        }
    }
}

#[derive(Deserialize)]
pub struct GetBoostedArgs {
    fields: Option<String>,
}

#[get("/boosted")]
pub async fn get_boosted(
    req: HttpRequest,
    args: Query<GetBoostedArgs>,
    pool: Data<Pool>,
) -> Res<Vec<JsonObject>> {
    let fields: Vec<&str> = args.fields.as_deref().unwrap_or("").split(',').collect();
    let updated_since = OffsetDateTime::UNIX_EPOCH;
    let include_deleted = false;

    let items =
        db::element::queries::select_updated_since(updated_since, None, include_deleted, &pool)
            .await
            .map_err(|_| RestApiError::database())?;

    req.extensions_mut()
        .insert(RequestExtension::new(items.len()));

    let items = items
        .into_iter()
        .filter(|it| match it.tags.get("boost:expires") {
            Some(boost_expires) => match boost_expires.as_str() {
                Some(boost_expires) => match OffsetDateTime::parse(boost_expires, &Rfc3339) {
                    Ok(boost_expires) => boost_expires > OffsetDateTime::now_utc(),
                    Err(_) => false,
                },
                None => false,
            },
            None => false,
        })
        .map(|it| service::element::generate_tags(&it, &fields))
        .collect();

    Ok(Json(items))
}

#[derive(Serialize)]
pub struct PendingPlace {
    pub id: i64,
    pub lat: f64,
    pub lon: f64,
    pub icon: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_hours: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<i64>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub verified_at: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub osm_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facebook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instagram: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub boosted_until: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_app_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
}

#[get("/pending")]
pub async fn get_pending(pool: Data<Pool>) -> Res<Vec<PendingPlace>> {
    let items = db::place_submission::queries::select_open_and_not_revoked(&pool)
        .await
        .map_err(|_| RestApiError::database())?;
    let items: Vec<PendingPlace> = items
        .into_iter()
        .map(|it| PendingPlace {
            id: it.id,
            lat: it.lat,
            lon: it.lon,
            icon: it.icon(),
            name: it.name.clone(),
            address: it.address(),
            opening_hours: it.opening_hours(),
            comments: None,
            created_at: it.created_at,
            updated_at: it.updated_at,
            verified_at: Some(it.created_at),
            osm_id: None,
            phone: it.phone(),
            website: it.website(),
            twitter: it.twitter(),
            facebook: it.facebook(),
            instagram: it.instagram(),
            line: it.line(),
            email: it.email(),
            boosted_until: None,
            required_app_url: None,
            description: it.description(),
            image: it.image(),
            payment_provider: it.payment_provider(),
            pending: Some(true),
        })
        .collect();
    Ok(Json(items))
}

#[derive(Deserialize)]
pub struct SearchArgs {
    lat: Option<f64>,
    lon: Option<f64>,
    radius_km: Option<f64>,
    name: Option<String>,
    payment_provider: Option<String>,
    include_pending: Option<bool>,
    prevent_pending_id_clash: Option<bool>,
}

#[derive(Serialize)]
pub struct SearchedPlace {
    pub id: i64,
    pub lat: f64,
    pub lon: f64,
    pub icon: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_hours: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<i64>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub verified_at: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub osm_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facebook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instagram: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub boosted_until: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_app_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
}

#[get("/search")]
pub async fn search(args: Query<SearchArgs>, pool: Data<Pool>) -> Res<Vec<SearchedPlace>> {
    let lat = args.lat.unwrap_or(0.0);
    let lon = args.lon.unwrap_or(0.0);
    let radius_km = args.radius_km.unwrap_or(100_000.0);
    let name = args.name.clone().unwrap_or("".to_string());
    let payment_provider = args.payment_provider.clone().unwrap_or("".to_string());
    let include_pending = args.include_pending.unwrap_or(false);

    let payment_provider_whitelist = vec!["coinos".to_string(), "square".to_string()];

    if !payment_provider.is_empty() && !payment_provider_whitelist.contains(&payment_provider) {
        return Err(RestApiError {
            code: crate::rest::error::RestApiErrorCode::InvalidInput,
            message: "Unknown payment provider".to_string(),
        });
    }

    let lat_radius = radius_km / 111.0;
    let lon_radius = radius_km / (111.0 * lat.to_radians().cos());

    let mut min_lat = lat - lat_radius;
    let mut max_lat = lat + lat_radius;
    let mut min_lon = lon - lon_radius;
    let mut max_lon = lon + lon_radius;

    if min_lat < -90.0 {
        min_lat = -90.0;
    }

    if max_lat > 90.0 {
        max_lat = 90.0;
    }

    if min_lon < -180.0 {
        min_lon = -180.0;
    }

    if max_lon > 180.0 {
        max_lon = 180.0;
    }

    let global = min_lat == -90.0 && max_lat == 90.0 && min_lon == -180.0 && max_lon == 180.0;

    let mut filters_applied = 0;
    let mut matches = vec![];
    let mut pending_matches = vec![];

    if !global {
        matches = db::element::queries::select_by_bbox(min_lat, max_lat, min_lon, max_lon, &pool)
            .await
            .map_err(|_| RestApiError::database())?;
        if include_pending {
            pending_matches = db::place_submission::queries::select_by_bbox(
                min_lat, max_lat, min_lon, max_lon, &pool,
            )
            .await
            .map_err(|_| RestApiError::database())?;
        }
        filters_applied += 1;
    }

    if !name.is_empty() {
        if filters_applied == 0 {
            matches = db::element::queries::select_by_search_query(&name, false, &pool)
                .await
                .map_err(|_| RestApiError::database())?;
            if include_pending {
                pending_matches =
                    db::place_submission::queries::select_by_search_query(name, false, &pool)
                        .await
                        .map_err(|_| RestApiError::database())?;
            }
            filters_applied += 1;
        } else {
            matches = matches
                .into_iter()
                .filter(|it| it.name().to_lowercase().contains(&name))
                .collect();
            if include_pending {
                pending_matches = pending_matches
                    .into_iter()
                    .filter(|it| it.name.to_lowercase().contains(&name))
                    .collect();
            }
        }

        filters_applied += 1;
    }

    if !payment_provider.is_empty() {
        if filters_applied == 0 {
            matches =
                db::element::queries::select_by_payment_provider(payment_provider.clone(), &pool)
                    .await
                    .map_err(|_| RestApiError::database())?;
            if include_pending {
                pending_matches =
                    db::place_submission::queries::select_by_origin(payment_provider, &pool)
                        .await
                        .map_err(|_| RestApiError::database())?;
            }
        } else {
            matches = matches
                .into_iter()
                .filter(|it| it.supports_payment_provider(&payment_provider))
                .collect();
            if include_pending {
                pending_matches = pending_matches
                    .into_iter()
                    .filter(|it| it.origin == payment_provider)
                    .collect();
            }
        }
    }

    let mut matches: Vec<SearchedPlace> = matches.into_iter().map(Into::into).collect();
    let mut pending_matches: Vec<SearchedPlace> = pending_matches
        .into_iter()
        .map(|it| {
            let id = if args.prevent_pending_id_clash.unwrap_or(true) {
                100_000_000 + it.id
            } else {
                it.id
            };
            SearchedPlace {
                id,
                lat: it.lat,
                lon: it.lon,
                icon: it.icon(),
                name: it.name.clone(),
                address: it.address(),
                opening_hours: it.opening_hours(),
                comments: None,
                created_at: it.created_at,
                updated_at: it.updated_at,
                verified_at: Some(it.created_at),
                osm_id: None,
                phone: it.phone(),
                website: it.website(),
                twitter: it.twitter(),
                facebook: it.facebook(),
                instagram: it.instagram(),
                line: it.line(),
                email: it.email(),
                boosted_until: None,
                required_app_url: None,
                description: it.description(),
                image: it.image(),
                payment_provider: it.payment_provider(),
                pending: Some(true),
            }
        })
        .collect();

    let mut res: Vec<SearchedPlace> = vec![];
    res.append(&mut matches);
    res.append(&mut pending_matches);

    Ok(Json(res))
}

impl From<Element> for SearchedPlace {
    fn from(it: Element) -> Self {
        let comments = it.comment_count();
        let comments = if comments > 0 { Some(comments) } else { None };

        SearchedPlace {
            id: it.id,
            lat: it.lat.unwrap(),
            lon: it.lon.unwrap(),
            icon: it.icon("store"),
            name: it.name(),
            address: it.address(),
            opening_hours: it.opening_hours(),
            comments,
            created_at: it.created_at,
            updated_at: it.updated_at,
            verified_at: it.verified_at(),
            osm_id: Some(it.osm_id()),
            phone: it.phone(),
            website: it.website(),
            twitter: it.twitter(),
            facebook: it.facebook(),
            instagram: it.instagram(),
            line: it.line(),
            email: it.email(),
            boosted_until: it.boosted_until(),
            required_app_url: it.required_app_url(),
            description: it.description(),
            image: it.image(),
            payment_provider: it.payment_provider(),
            pending: None,
        }
    }
}

#[get("{id}")]
pub async fn get_by_id(
    id: Path<String>,
    args: Query<GetSingleArgs>,
    pool: Data<Pool>,
) -> Res<JsonObject> {
    let fields: Vec<&str> = args.fields.as_deref().unwrap_or("").split(',').collect();
    db::element::queries::select_by_id_or_osm_id(id.into_inner(), &pool)
        .await
        .map(|it| Json(service::element::generate_tags(&it, &fields)))
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })
}

#[derive(Serialize)]
pub struct Comment {
    pub id: i64,
    pub text: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl From<ElementComment> for Comment {
    fn from(val: ElementComment) -> Self {
        Comment {
            id: val.id,
            text: val.comment,
            created_at: val.created_at,
        }
    }
}

#[get("{id}/comments")]
pub async fn get_by_id_comments(id: Path<String>, pool: Data<Pool>) -> Res<Vec<Comment>> {
    let element = db::element::queries::select_by_id_or_osm_id(id.as_str(), &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;
    db::element_comment::queries::select_by_element_id(element.id, false, i64::MAX, &pool)
        .await
        .map(|it| Json(it.into_iter().map(Comment::from).collect()))
        .map_err(|_| RestApiError::database())
}

#[cfg(test)]
mod test {
    use crate::db::test::pool;
    use crate::service::overpass::OverpassElement;
    use crate::{db, Result};
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use geojson::JsonObject;
    use serde_json::{Map, Value};
    use time::macros::datetime;

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
        let element = db::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(element.id, res.first().unwrap()["id"].as_i64().unwrap());
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let pool = pool();
        let _element_1 = db::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let _element_2 = db::element::queries::insert(OverpassElement::mock(2), &pool).await?;
        let _element_3 = db::element::queries::insert(OverpassElement::mock(3), &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=2").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, res.len());
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let pool = pool();
        let element_1 = db::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        db::element::queries::set_updated_at(element_1.id, datetime!(2022-01-05 00:00 UTC), &pool)
            .await?;
        let element_2 = db::element::queries::insert(OverpassElement::mock(2), &pool).await?;
        let _element_2 = db::element::queries::set_updated_at(
            element_2.id,
            datetime!(2022-02-05 00:00 UTC),
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2022-01-10T00:00:00Z&limit=100")
            .to_request();
        let res: Vec<Map<String, Value>> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        Ok(())
    }

    #[test]
    async fn get_by_id() -> Result<()> {
        let pool = pool();
        let element = db::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_id),
        )
        .await;
        let req = TestRequest::get().uri("/1").to_request();
        let res: JsonObject = test::call_and_read_body_json(&app, req).await;
        assert_eq!(element.id, res["id"].as_i64().unwrap());
        Ok(())
    }
}
