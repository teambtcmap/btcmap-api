use crate::db;
use crate::db::main::element::schema::Element;
use crate::db::main::element_comment::schema::ElementComment;
use crate::db::main::element_event::queries::ElementEventWithUser;
use crate::db::main::place_submission::schema::PlaceSubmission;
use crate::db::main::MainPool;
use crate::rest::auth::Auth;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult as Res;
use crate::service;
use crate::Error;
use actix_web::delete;
use actix_web::get;
use actix_web::post;
use actix_web::put;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use geojson::JsonObject;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use time::OffsetDateTime;
use tracing::warn;

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
    lang: Option<String>,
}

#[derive(Deserialize)]
pub struct GetSingleArgs {
    fields: Option<String>,
    lang: Option<String>,
}

#[get("")]
pub async fn get(args: Query<GetListArgs>, pool: Data<MainPool>) -> Res<Vec<JsonObject>> {
    let fields: Vec<&str> = args.fields.as_deref().unwrap_or("").split(',').collect();
    let updated_since = args.updated_since.unwrap_or(OffsetDateTime::UNIX_EPOCH);
    let include_deleted = args.include_deleted.unwrap_or(false) || fields.contains(&"deleted_at");
    let include_pending = args.include_pending.unwrap_or(false);
    let prevent_pending_id_clash = args.prevent_pending_id_clash.unwrap_or(true);
    let lang = args
        .lang
        .as_deref()
        .map(|l| &l[..2.min(l.len())])
        .unwrap_or("en");

    let elements = db::main::element::queries::select_updated_since(
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
            &mut db::main::place_submission::queries::select_updated_since(
                updated_since,
                args.limit,
                include_deleted,
                &pool,
            )
            .await
            .map_err(|_| RestApiError::database())?,
        );
    }

    if submissions.is_empty() {
        let elements = elements
            .into_iter()
            .map(|it| service::element::generate_tags(&it, &fields, Some(lang)))
            .collect();

        Ok(Json(elements))
    } else {
        let mut items: Vec<GetItem> = vec![];
        let mut elements: Vec<GetItem> = elements
            .into_iter()
            .map(|e| GetItem::Element(Box::new(e)))
            .collect();
        let mut submissions: Vec<GetItem> = submissions
            .into_iter()
            .map(|s| GetItem::PlaceSubmission(Box::new(s)))
            .collect();
        items.append(&mut elements);
        items.append(&mut submissions);

        items.sort_by_key(|a| a.updated_at());

        Ok(Json(
            items
                .into_iter()
                .map(|it| it.to_json(&fields, prevent_pending_id_clash, Some(lang)))
                .take(args.limit.unwrap_or(i64::MAX) as usize)
                .collect(),
        ))
    }
}

pub enum GetItem {
    Element(Box<Element>),
    PlaceSubmission(Box<PlaceSubmission>),
}

impl GetItem {
    fn updated_at(&self) -> OffsetDateTime {
        match self {
            GetItem::Element(element) => element.updated_at,
            GetItem::PlaceSubmission(place_submission) => place_submission.updated_at,
        }
    }

    fn to_json(
        &self,
        fields: &Vec<&str>,
        prevent_pending_id_clash: bool,
        lang: Option<&str>,
    ) -> JsonObject {
        match self {
            GetItem::Element(element) => service::element::generate_tags(element, fields, lang),
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
pub async fn get_pending(pool: Data<MainPool>) -> Res<Vec<PendingPlace>> {
    let items = db::main::place_submission::queries::select_open_and_not_revoked(&pool)
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
    tag_name: Option<String>,
    tag_value: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localized_name: Option<Map<String, Value>>,
}

#[get("/search")]
pub async fn search(args: Query<SearchArgs>, pool: Data<MainPool>) -> Res<Vec<SearchedPlace>> {
    let lat = args.lat.unwrap_or(0.0);
    let lon = args.lon.unwrap_or(0.0);
    let radius_km = args.radius_km.unwrap_or(100_000.0);
    let name = args.name.clone().unwrap_or("".to_string());
    let tag_name = args.tag_name.clone().unwrap_or("".to_string());
    let tag_value = args.tag_value.clone().unwrap_or("".to_string());
    let include_pending = args.include_pending.unwrap_or(false);

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
        matches =
            db::main::element::queries::select_by_bbox(min_lat, max_lat, min_lon, max_lon, &pool)
                .await
                .map_err(|_| RestApiError::database())?;
        if include_pending {
            pending_matches = db::main::place_submission::queries::select_pending_by_bbox(
                min_lat, max_lat, min_lon, max_lon, &pool,
            )
            .await
            .map_err(|_| RestApiError::database())?;
        }
        filters_applied += 1;
    }

    if !name.is_empty() {
        if filters_applied == 0 {
            matches = db::main::element::queries::select_by_search_query(&name, false, &pool)
                .await
                .map_err(|_| RestApiError::database())?;
            if include_pending {
                pending_matches =
                    db::main::place_submission::queries::select_by_search_query(name, false, &pool)
                        .await
                        .map_err(|_| RestApiError::database())?;
            }
            filters_applied += 1;
        } else {
            matches.retain(|it| it.name(Some("en")).to_lowercase().contains(&name));
            if include_pending {
                pending_matches.retain(|it| it.name.to_lowercase().contains(&name));
            }
        }

        filters_applied += 1;
    }

    if !tag_name.is_empty() && !tag_value.is_empty() {
        if filters_applied == 0 {
            matches = db::main::element::queries::select_by_osm_tag_value(
                tag_name.clone(),
                tag_value.clone(),
                &pool,
            )
            .await
            .map_err(|_| RestApiError::database())?;
            if include_pending {
                let origin = if tag_name == "payment:lightning:operator" && tag_value == "square" {
                    "square"
                } else {
                    ""
                };

                if !origin.is_empty() {
                    pending_matches = db::main::place_submission::queries::select_by_origin(
                        origin.to_string(),
                        &pool,
                    )
                    .await
                    .map_err(|_| RestApiError::database())?;
                }
            }
        } else {
            matches.retain(|it| match &it.overpass_data.tags {
                Some(tags) => match tags.get(&tag_name) {
                    Some(tag) => tag.as_str() == Some(&tag_value),
                    None => false,
                },
                None => false,
            });
            if include_pending {
                let origin = if tag_name == "payment:lightning:operator" && tag_value == "square" {
                    "square"
                } else {
                    ""
                };

                if !origin.is_empty() {
                    pending_matches.retain(|it| it.origin == origin);
                }
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
                localized_name: None,
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

        let mut localized_name: Option<Map<String, Value>> = None;
        if let Some(ref tags) = it.overpass_data.tags {
            let mut localized = Map::new();
            for (key, value) in tags {
                if let Some(lang_code) = key.strip_prefix("name:") {
                    if lang_code.len() == 2 && value.is_string() {
                        localized.insert(lang_code.to_string(), value.clone());
                    }
                }
            }
            if !localized.is_empty() {
                localized_name = Some(localized);
            }
        }

        SearchedPlace {
            id: it.id,
            lat: it.lat.unwrap(),
            lon: it.lon.unwrap(),
            icon: it.icon("store"),
            name: it.name(Some("en")),
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
            localized_name,
        }
    }
}

#[get("{id}")]
pub async fn get_by_id(
    id: Path<String>,
    args: Query<GetSingleArgs>,
    pool: Data<MainPool>,
) -> Res<JsonObject> {
    let fields: Vec<&str> = args.fields.as_deref().unwrap_or("").split(',').collect();
    let lang = args
        .lang
        .as_deref()
        .map(|l| &l[..2.min(l.len())])
        .unwrap_or("en");
    db::main::element::queries::select_by_id_or_osm_id(id.into_inner(), &pool)
        .await
        .map(|it| Json(service::element::generate_tags(&it, &fields, Some(lang))))
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
pub async fn get_by_id_comments(id: Path<String>, pool: Data<MainPool>) -> Res<Vec<Comment>> {
    let element = db::main::element::queries::select_by_id_or_osm_id(id.as_str(), &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;
    db::main::element_comment::queries::select_by_element_id(element.id, false, i64::MAX, &pool)
        .await
        .map(|it| Json(it.into_iter().map(Comment::from).collect()))
        .map_err(|_| RestApiError::database())
}

#[derive(Serialize)]
pub struct Activity {
    pub id: i64,
    pub r#type: String,
    pub user_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_tip: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<ElementEventWithUser> for Activity {
    fn from(val: ElementEventWithUser) -> Self {
        let re = regex::Regex::new(r"(lightning:[^)]+)").unwrap();
        let tip = re.captures(&val.user_description).map(|c| c[1].to_string());

        Activity {
            id: val.id,
            r#type: val.r#type,
            user_id: Some(val.user_id),
            user_name: Some(val.user_name),
            user_tip: tip,
            created_at: val.created_at,
            updated_at: val.updated_at,
        }
    }
}

#[get("{id}/activity")]
pub async fn get_by_id_activity(id: Path<String>, pool: Data<MainPool>) -> Res<Vec<Activity>> {
    let element = db::main::element::queries::select_by_id_or_osm_id(id.as_str(), &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;
    db::main::element_event::queries::select_by_element_id(element.id, &pool)
        .await
        .map(|it| Json(it.into_iter().map(Activity::from).collect()))
        .map_err(|_| RestApiError::database())
}

#[derive(Serialize)]
pub struct AreaResponse {
    pub id: i64,
    pub alias: String,
    pub tags: Map<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Deserialize)]
pub struct GetAreasArgs {
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    include_deleted: Option<bool>,
}

#[get("{id}/areas")]
pub async fn get_by_id_areas(
    id: Path<String>,
    args: Query<GetAreasArgs>,
    pool: Data<MainPool>,
) -> Res<Vec<AreaResponse>> {
    let element = db::main::element::queries::select_by_id_or_osm_id(id.as_str(), &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;

    let area_elements = db::main::area_element::queries::select_by_element_id(element.id, &pool)
        .await
        .map_err(|_| RestApiError::database())?;

    let mut areas: Vec<AreaResponse> = vec![];

    for area_element in area_elements {
        if let Ok(area) = db::main::area::queries::select_by_id(area_element.area_id, &pool).await {
            if !args.include_deleted.unwrap_or(false) && area.deleted_at.is_some() {
                continue;
            }

            if let Some(ref type_filter) = args.r#type {
                let area_type = area.tags.get("type").and_then(|v| v.as_str());

                if area_type != Some(type_filter.as_str()) {
                    continue;
                }
            }

            let mut tags = area.tags;
            tags.remove("geo_json");
            areas.push(AreaResponse {
                id: area.id,
                alias: area.alias,
                tags,
                created_at: area.created_at,
                updated_at: area.updated_at,
            });
        }
    }

    Ok(Json(areas))
}

#[get("/saved")]
pub async fn get_saved(auth: Auth, pool: Data<MainPool>) -> Res<Vec<JsonObject>> {
    warn!("PRE USER");
    let user = auth.user.ok_or(RestApiError::unauthorized())?;
    warn!("USER LOADED");
    let elements = db::main::element::queries::select_by_ids(&user.saved_places, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    warn!("ELEMENTS LOADED");
    let items: Vec<JsonObject> = elements
        .into_iter()
        .map(|e| service::element::generate_tags(&e, &["name", "lat", "lon"], None))
        .collect();
    Ok(Json(items))
}

#[put("/saved")]
pub async fn put_saved(auth: Auth, args: Json<Vec<i64>>, pool: Data<MainPool>) -> Res<Vec<i64>> {
    let user = auth.user.ok_or(RestApiError::unauthorized())?;
    db::main::user::queries::set_saved_places(user.id, &args, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(actix_web::web::Json(args.into_inner()))
}

#[post("/saved")]
pub async fn post_saved(auth: Auth, args: Json<i64>, pool: Data<MainPool>) -> Res<Vec<i64>> {
    let user = auth.user.ok_or(RestApiError::unauthorized())?;
    let mut saved_places = user.saved_places.clone();
    if !saved_places.contains(&args) {
        saved_places.push(args.into_inner());
    }
    db::main::user::queries::set_saved_places(user.id, &saved_places, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(actix_web::web::Json(saved_places))
}

#[delete("/saved/{id}")]
pub async fn delete_saved(auth: Auth, path: Path<i64>, pool: Data<MainPool>) -> Res<Vec<i64>> {
    let user = auth.user.ok_or(RestApiError::unauthorized())?;
    let id = path.into_inner();
    let mut saved_places = user.saved_places.clone();
    saved_places.retain(|x| *x != id);
    db::main::user::queries::set_saved_places(user.id, &saved_places, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(actix_web::web::Json(saved_places))
}

#[cfg(test)]
mod test {
    use crate::db::main::area::schema::Area;
    use crate::db::main::test::pool;
    use crate::service::overpass::OverpassElement;
    use crate::{db, Result};
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use geojson::JsonObject;
    use serde_json::{Map, Value};
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
        let element = db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
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
        let _element_1 =
            db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let _element_2 =
            db::main::element::queries::insert(OverpassElement::mock(2), &pool).await?;
        let _element_3 =
            db::main::element::queries::insert(OverpassElement::mock(3), &pool).await?;
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
        let element_1 = db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        db::main::element::queries::set_updated_at(
            element_1.id,
            datetime!(2022-01-05 00:00 UTC),
            &pool,
        )
        .await?;
        let element_2 = db::main::element::queries::insert(OverpassElement::mock(2), &pool).await?;
        let _element_2 = db::main::element::queries::set_updated_at(
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
        let element = db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
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

    #[test]
    async fn get_by_id_areas_excludes_deleted_by_default() -> Result<()> {
        let pool = pool();
        let element = db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let area = db::main::area::queries::insert(Area::mock_tags(), &pool).await?;
        db::main::area_element::queries::insert(area.id, element.id, &pool).await?;
        db::main::area::queries::set_deleted_at(area.id, Some(OffsetDateTime::now_utc()), &pool)
            .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_id_areas),
        )
        .await;
        let req = TestRequest::get().uri("/1/areas").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_by_id_areas_includes_deleted_when_requested() -> Result<()> {
        let pool = pool();
        let element = db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let area = db::main::area::queries::insert(Area::mock_tags(), &pool).await?;
        db::main::area_element::queries::insert(area.id, element.id, &pool).await?;
        db::main::area::queries::set_deleted_at(area.id, Some(OffsetDateTime::now_utc()), &pool)
            .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_id_areas),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/areas?include_deleted=true", element.id))
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(area.id, res[0]["id"].as_i64().unwrap());
        Ok(())
    }
}
