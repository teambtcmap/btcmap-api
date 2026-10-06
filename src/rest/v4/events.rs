use crate::db;
use crate::db::main::event::schema::Event;
use crate::db::main::event::schema::Status;
use crate::db::main::user::schema::Role;
use crate::db::main::MainPool;
use crate::rest::auth::Auth;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult;
use crate::service::timezone;
use crate::service::timezone::EventTime;
use crate::Error;
use actix_web::delete;
use actix_web::get;
use actix_web::post;
use actix_web::put;
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
use std::collections::HashMap;
use std::collections::HashSet;
use time::format_description::well_known::Rfc3339;
use time::macros::datetime;
use time::OffsetDateTime;

/// Submitter of an event, exposed as a nested `submitted_by` object on every
/// v4 event response.
#[derive(Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, rename = "EventAuthor")]
pub struct Author {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, rename = "Event")]
pub struct Item {
    #[ts(type = "number")]
    pub id: i64,
    pub lat: f64,
    pub lon: f64,
    pub name: String,
    pub website: String,
    /// Review state: `pending`, `live` or `rejected`. All v4 event responses
    /// expose it so clients can badge unreviewed submissions.
    pub status: String,
    /// Submitter of the event, when known. Omitted for events created before
    /// submissions were attributed and for RPC-created events.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_by: Option<Author>,
    #[ts(type = "string")]
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[ts(type = "string", optional)]
    #[serde(
        with = "time::serde::rfc3339::option",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ends_at: Option<OffsetDateTime>,
    // Present only on delta responses (`updated_since` supplied) so a sync
    // client can advance its cursor. Omitted from the legacy full snapshot.
    #[ts(type = "string", optional)]
    #[serde(
        with = "time::serde::rfc3339::option",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub updated_at: Option<OffsetDateTime>,
    // Present only on delta responses that include tombstones
    // (`include_deleted=true`). Omitted otherwise, including the legacy
    // full snapshot.
    #[ts(type = "string", optional)]
    #[serde(
        with = "time::serde::rfc3339::option",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<Event> for Item {
    fn from(val: Event) -> Self {
        Item {
            id: val.id,
            lat: val.lat,
            lon: val.lon,
            name: val.name,
            website: val.website,
            status: val.status.to_string(),
            submitted_by: None,
            starts_at: val.starts_at,
            ends_at: val.ends_at,
            updated_at: None,
            deleted_at: None,
        }
    }
}

impl Item {
    /// Delta representation: exposes `updated_at` (sync cursor) and
    /// `deleted_at` (tombstone). Non-deleted rows serialize `deleted_at` as
    /// absent because of `skip_serializing_if`.
    fn from_delta(val: Event) -> Self {
        Item {
            updated_at: Some(val.updated_at),
            deleted_at: val.deleted_at,
            ..Item::from(val)
        }
    }
}

/// Resolve the `{id, name}` author for every distinct `submitted_by` id in the
/// supplied iterator. Ids without a matching user (or `None`) are dropped.
pub(crate) async fn resolve_authors<I>(
    submitted_by: I,
    pool: &MainPool,
) -> crate::Result<HashMap<i64, Author>>
where
    I: IntoIterator<Item = Option<i64>>,
{
    let ids: Vec<i64> = submitted_by
        .into_iter()
        .flatten()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    Ok(db::main::user::queries::select_by_ids(&ids, pool)
        .await?
        .into_iter()
        .map(|user| {
            (
                user.id,
                Author {
                    id: user.id,
                    name: user.name,
                },
            )
        })
        .collect())
}

/// Build an [`Item`], attaching the resolved submitter author (if any).
pub(crate) fn with_author(event: &Event, delta: bool, authors: &HashMap<i64, Author>) -> Item {
    let author = event.submitted_by.and_then(|id| authors.get(&id).cloned());
    let mut item = if delta {
        Item::from_delta(event.clone())
    } else {
        Item::from(event.clone())
    };
    item.submitted_by = author;
    item
}

/// Enrich a batch of events with their resolved submitter authors.
async fn attach_authors(
    events: Vec<Event>,
    delta: bool,
    pool: &MainPool,
) -> Result<Vec<Item>, RestApiError> {
    let authors = resolve_authors(events.iter().map(|it| it.submitted_by), pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(events
        .iter()
        .map(|event| with_author(event, delta, &authors))
        .collect())
}

/// Enrich a single event with its resolved submitter author.
async fn attach_author(event: Event, pool: &MainPool) -> Result<Item, RestApiError> {
    attach_authors(vec![event], false, pool)
        .await?
        .pop()
        .ok_or_else(RestApiError::database)
}

#[derive(Deserialize)]
pub struct GetByAreaArgs {
    pub from: Option<String>,
    pub to: Option<String>,
    /// Comma-separated list of statuses to include, or `all`. Defaults to
    /// `live` only.
    pub status: Option<String>,
}

#[derive(Deserialize)]
pub struct GetArgs {
    /// When present, switch to delta semantics: return every row with
    /// `updated_at` after this instant, including past events, so a sync
    /// client can page through the change log. When absent, the legacy full
    /// snapshot is returned unchanged.
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    updated_since: Option<OffsetDateTime>,
    limit: Option<i64>,
    include_deleted: Option<bool>,
    /// Comma-separated list of statuses to include, or `all`. Defaults to
    /// `live` only so clients never observe unreviewed submissions unless they
    /// explicitly opt in.
    status: Option<String>,
}

/// Query string for endpoints that only need the status opt-in.
#[derive(Deserialize)]
pub struct StatusArgs {
    /// Comma-separated list of statuses to include, or `all`. Defaults to
    /// `live` only.
    pub status: Option<String>,
}

/// Parse the `status` query parameter into the set of statuses a read request
/// is allowed to return. Absent means "public view": only `live`. `all`
/// expands to every status, and any comma-separated subset of
/// `pending`/`live`/`rejected` is accepted so a caching client can opt in to
/// exactly what it understands.
fn parse_statuses(raw: Option<&str>) -> Result<Vec<Status>, RestApiError> {
    let raw = match raw {
        None => return Ok(vec![Status::Live]),
        Some(raw) => raw.trim(),
    };
    if raw.is_empty() {
        return Err(RestApiError::invalid_input(
            "status must be a comma-separated list of pending, live, rejected, or 'all'",
        ));
    }
    if raw.eq_ignore_ascii_case("all") {
        return Ok(Status::ALL.to_vec());
    }
    let mut statuses: Vec<Status> = Vec::new();
    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let status = part
            .parse::<Status>()
            .map_err(RestApiError::invalid_input)?;
        if !statuses.contains(&status) {
            statuses.push(status);
        }
    }
    if statuses.is_empty() {
        return Err(RestApiError::invalid_input(
            "status must be a comma-separated list of pending, live, rejected, or 'all'",
        ));
    }
    Ok(statuses)
}

#[get("")]
pub async fn get(args: Query<GetArgs>, pool: Data<MainPool>) -> RestResult<Vec<Item>> {
    let statuses = parse_statuses(args.status.as_deref())?;
    match args.updated_since {
        // Legacy full snapshot: identical to the pre-delta response. Existing
        // clients that don't request `updated_since` keep seeing only upcoming,
        // non-deleted events and no `updated_at`/`deleted_at` fields. Pending
        // and rejected submissions are hidden unless `status` opts in.
        None => {
            let items = db::main::event::queries::select_all(&pool)
                .await
                .map_err(|_| RestApiError::database())?;
            let now = OffsetDateTime::now_utc();
            let items: Vec<Event> = items
                .into_iter()
                .filter(|it| {
                    it.deleted_at.is_none() && it.starts_at > now && statuses.contains(&it.status)
                })
                .collect();
            Ok(Json(attach_authors(items, false, &pool).await?))
        }
        // Delta change log: deliberately skip the upcoming filter so edits and
        // tombstones of already-started events are still observable. The client
        // prunes past events locally. The status filter is applied in SQL so the
        // page limit still counts the rows the caller actually wants.
        Some(updated_since) => {
            let items = db::main::event::queries::select_updated_since(
                updated_since,
                args.include_deleted.unwrap_or(false),
                statuses,
                args.limit,
                &pool,
            )
            .await
            .map_err(|_| RestApiError::database())?;
            Ok(Json(attach_authors(items, true, &pool).await?))
        }
    }
}

#[get("{id}")]
pub async fn get_by_id(
    id: Path<i64>,
    args: Query<StatusArgs>,
    pool: Data<MainPool>,
) -> RestResult<Item> {
    let statuses = parse_statuses(args.status.as_deref())?;
    let event = db::main::event::queries::select_by_id(id.into_inner(), &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;
    // A pending/rejected event is invisible to clients that did not opt in, so
    // it reads as "not found" rather than leaking its existence.
    if !statuses.contains(&event.status) {
        return Err(RestApiError::not_found());
    }
    Ok(Json(attach_author(event, &pool).await?))
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

    let statuses = parse_statuses(args.status.as_deref())?;

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
    let items: Vec<Event> = candidates
        .into_iter()
        .filter(|event| {
            if event.deleted_at.is_some() {
                return false;
            }
            if event.starts_at < from || event.starts_at > to {
                return false;
            }
            if !statuses.contains(&event.status) {
                return false;
            }
            event_point_in_geometries(event.lon, event.lat, &geometries)
        })
        .collect();

    Ok(Json(attach_authors(items, false, &pool).await?))
}

/// Roles allowed to bypass review: their submissions go straight to `live` and
/// they may transition an event's status.
fn is_privileged(auth: &Auth) -> bool {
    auth.effective_roles()
        .iter()
        .any(|role| matches!(role, Role::EventManager | Role::Admin | Role::Root))
}

#[derive(Deserialize, ts_rs::TS)]
#[ts(export, rename = "PostEventArgs")]
pub struct PostArgs {
    pub lat: f64,
    pub lon: f64,
    pub name: String,
    pub website: String,
    /// RFC 3339 with an offset, or a floating local datetime that needs
    /// `timezone` to be placed on the timeline.
    #[ts(type = "string")]
    pub starts_at: EventTime,
    #[serde(default)]
    #[ts(type = "string", optional)]
    pub ends_at: Option<EventTime>,
    /// Either `"auto"` (infer from lat/lon) or an IANA zone name.
    #[serde(default)]
    pub timezone: Option<String>,
    #[ts(type = "number | null")]
    pub area_id: Option<i64>,
}

/// `POST /v4/events`
///
/// Submits a new event. Authenticated users without an event-manager, admin or
/// root role create `pending` events that an admin must review; privileged
/// users create `live` events and are bound by their geofence, exactly like the
/// RPC `create_event`.
#[post("")]
pub async fn post(auth: Auth, args: Json<PostArgs>, pool: Data<MainPool>) -> RestResult<Item> {
    let privileged = is_privileged(&auth);
    let user = auth.user.ok_or_else(RestApiError::unauthorized)?;

    if !(-90.0..=90.0).contains(&args.lat) {
        return Err(RestApiError::invalid_input(
            "Latitude must be between -90 and 90",
        ));
    }

    if !(-180.0..=180.0).contains(&args.lon) {
        return Err(RestApiError::invalid_input(
            "Longitude must be between -180 and 180",
        ));
    }

    if args.name.trim().is_empty() {
        return Err(RestApiError::invalid_input("name cannot be empty"));
    }

    if privileged {
        crate::service::geofence::check(&user, args.lat, args.lon, &pool)
            .await
            .map_err(|_| RestApiError::forbidden())?;
    }

    let (starts_at, ends_at, _timezone) = timezone::resolve_create_times(
        args.starts_at,
        args.ends_at,
        args.timezone.as_deref(),
        args.lat,
        args.lon,
    )
    .map_err(|e| RestApiError::invalid_input(e.to_string()))?;

    let status = if privileged {
        Status::Live
    } else {
        Status::Pending
    };
    let event = db::main::event::queries::insert_with_status(
        args.area_id,
        args.lat,
        args.lon,
        args.name.clone(),
        args.website.clone(),
        starts_at,
        ends_at,
        status,
        Some(user.id),
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;
    Ok(Json(attach_author(event, &pool).await?))
}

/// `DELETE /v4/events/{id}`
///
/// Revokes the caller's own submission while it is still awaiting review, by
/// soft-deleting it. Only the original submitter may call this, and only for a
/// `pending` event; a `live` event must be taken down by an event manager,
/// admin or root through the RPC `delete_event`. Revoking an already-revoked
/// event succeeds as a no-op so the call is idempotent.
#[delete("{id}")]
pub async fn delete(id: Path<i64>, auth: Auth, pool: Data<MainPool>) -> RestResult<Item> {
    let user = auth.user.ok_or_else(RestApiError::unauthorized)?;
    let id = id.into_inner();

    let event = db::main::event::queries::select_by_id(id, &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;

    // Self-service revocation is limited to the submitter and to events that
    // are still awaiting review. Everything else reads as forbidden so the
    // endpoint can't be used to take down published events.
    if event.submitted_by != Some(user.id) || event.status != Status::Pending {
        return Err(RestApiError::forbidden());
    }

    let event =
        db::main::event::queries::set_deleted_at(id, Some(OffsetDateTime::now_utc()), &pool)
            .await
            .map_err(|_| RestApiError::database())?;
    Ok(Json(attach_author(event, &pool).await?))
}

#[derive(Deserialize, ts_rs::TS)]
#[ts(export, rename = "ChangeEventStatusArgs")]
pub struct ChangeStatusArgs {
    /// Target status. Only `live` and `rejected` are valid here; events are
    /// created as `pending` and never return to it through this endpoint.
    pub status: String,
}

/// `PUT /v4/events/{id}/status`
///
/// Approve or reject an event. Restricted to event managers, admins and roots,
/// and the event location must be inside the caller's geofence (when set).
#[put("{id}/status")]
pub async fn put_status(
    id: Path<i64>,
    auth: Auth,
    args: Json<ChangeStatusArgs>,
    pool: Data<MainPool>,
) -> RestResult<Item> {
    let user = auth.user.as_ref().ok_or_else(RestApiError::unauthorized)?;
    if !is_privileged(&auth) {
        return Err(RestApiError::forbidden());
    }

    let status = match args.status.as_str() {
        "live" => Status::Live,
        "rejected" => Status::Rejected,
        other => {
            return Err(RestApiError::invalid_input(format!(
                "status must be one of: live, rejected (got '{other}')"
            )))
        }
    };

    let id = id.into_inner();
    let event = db::main::event::queries::select_by_id(id, &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;

    crate::service::geofence::check(user, event.lat, event.lon, &pool)
        .await
        .map_err(|_| RestApiError::forbidden())?;

    let event = db::main::event::queries::set_status(id, status, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(Json(attach_author(event, &pool).await?))
}

/// `GET /v4/users/me/events`
///
/// Lists every non-deleted event submitted by the authenticated user, newest
/// first, including `pending` and `rejected` ones, so they can track review
/// status. Requires a Bearer token.
#[get("/me/events")]
pub async fn get_me(auth: Auth, pool: Data<MainPool>) -> RestResult<Vec<Item>> {
    let user = auth.user.ok_or_else(RestApiError::unauthorized)?;
    let events = db::main::event::queries::select_by_submitted_by(user.id, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(Json(attach_authors(events, false, &pool).await?))
}

/// Cheap two-stage membership check: assume the candidate point is outside
/// the area unless at least one of the area's geometries reports it as
/// contained. The bbox pre-filter has already discarded obvious misses; this
/// loop only runs on the survivors.
pub(crate) fn event_point_in_geometries(
    lon: f64,
    lat: f64,
    geometries: &[GeoJsonGeometry],
) -> bool {
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
    use crate::db::main::user::schema::Role;
    use crate::{db, Result};
    use actix_web::http::header;
    use actix_web::http::StatusCode;
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
            datetime!(2099-01-01 0:00 UTC),
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
        assert_eq!("live", res.first().unwrap()["status"].as_str().unwrap());
        assert!(res.first().unwrap().get("area_id").is_none());
        assert!(res.first().unwrap().get("ends_at").is_none());
        // Backward compatibility: the full snapshot must not grow the new
        // delta-only fields.
        assert!(res.first().unwrap().get("updated_at").is_none());
        assert!(res.first().unwrap().get("deleted_at").is_none());
        Ok(())
    }

    #[test]
    async fn get_updated_since_includes_past_events_unlike_snapshot() -> Result<()> {
        let pool = pool();
        let past = db::main::event::queries::insert(
            None,
            1.23,
            4.56,
            "past_event".to_string(),
            "https://example.com".to_string(),
            datetime!(2020-01-01 0:00 UTC),
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

        // Snapshot drops the past event...
        let req = TestRequest::get().uri("/").to_request();
        let snapshot: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(snapshot.is_empty());

        // ...while the delta change log keeps it and exposes updated_at.
        let req = TestRequest::get()
            .uri("/?updated_since=1970-01-01T00:00:00Z")
            .to_request();
        let delta: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, delta.len());
        assert_eq!(past.id, delta[0]["id"].as_i64().unwrap());
        assert!(delta[0].get("updated_at").is_some());
        assert!(delta[0].get("deleted_at").is_none());
        Ok(())
    }

    #[test]
    async fn get_updated_since_excludes_unchanged_events() -> Result<()> {
        let pool = pool();
        db::main::event::queries::insert(
            None,
            1.23,
            4.56,
            "future".to_string(),
            "https://example.com".to_string(),
            datetime!(2099-01-01 0:00 UTC),
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

        let req = TestRequest::get()
            .uri("/?updated_since=2100-01-01T00:00:00Z")
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_updated_since_deleted_only_with_include_deleted() -> Result<()> {
        let pool = pool();
        let event = db::main::event::queries::insert(
            None,
            1.23,
            4.56,
            "deleted".to_string(),
            "https://example.com".to_string(),
            datetime!(2099-01-01 0:00 UTC),
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

        // Delta without tombstones: the soft-deleted row is invisible.
        let req = TestRequest::get()
            .uri("/?updated_since=1970-01-01T00:00:00Z")
            .to_request();
        let hidden: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(hidden.is_empty());

        // Delta with tombstones: the row is returned with deleted_at set.
        let req = TestRequest::get()
            .uri("/?updated_since=1970-01-01T00:00:00Z&include_deleted=true")
            .to_request();
        let tombstones: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, tombstones.len());
        assert_eq!(event.id, tombstones[0]["id"].as_i64().unwrap());
        assert!(tombstones[0].get("deleted_at").is_some());
        Ok(())
    }

    #[test]
    async fn get_updated_since_respects_limit() -> Result<()> {
        let pool = pool();
        for name in ["one", "two", "three"] {
            db::main::event::queries::insert(
                None,
                1.23,
                4.56,
                name.to_string(),
                "https://example.com".to_string(),
                datetime!(2099-01-01 0:00 UTC),
                None,
                &pool,
            )
            .await?;
        }
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;

        let req = TestRequest::get()
            .uri("/?updated_since=1970-01-01T00:00:00Z&limit=2")
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, res.len());
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
            datetime!(2099-01-01 0:00 UTC),
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
            datetime!(2020-01-01 0:00 UTC),
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
            datetime!(2099-01-01 0:00 UTC),
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
            datetime!(2099-01-01 0:00 UTC),
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
        assert!(res.get("area_id").is_none());
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

    #[test]
    async fn get_by_id_hides_pending_unless_status_all() -> Result<()> {
        let pool = pool();
        let event = db::main::event::queries::insert_with_status(
            None,
            1.0,
            2.0,
            "pending".to_string(),
            "website".to_string(),
            datetime!(2999-01-01 0:00 UTC),
            None,
            crate::db::main::event::schema::Status::Pending,
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

        let req = TestRequest::get()
            .uri(&format!("/{}", event.id))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 404);

        let req = TestRequest::get()
            .uri(&format!("/{}?status=all", event.id))
            .to_request();
        let res: JsonObject = test::call_and_read_body_json(&app, req).await;
        assert_eq!("pending", res["status"].as_str().unwrap());
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
            datetime!(2099-01-01 0:00 UTC),
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
            datetime!(2099-01-01 0:00 UTC),
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
            datetime!(2099-01-01 0:00 UTC),
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
            datetime!(2020-01-01 0:00 UTC),
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
            datetime!(2020-01-01 0:00 UTC),
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
            datetime!(2099-01-01 0:00 UTC),
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
            datetime!(2024-06-01 0:00 UTC),
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
            datetime!(2099-01-01 0:00 UTC),
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
            datetime!(2099-01-01 0:00 UTC),
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
            datetime!(2099-01-01 0:00 UTC),
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
            datetime!(2099-01-01 0:00 UTC),
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

    async fn seed_user_with_token(
        roles: &[Role],
        geofence: &[i64],
        pool: &crate::db::main::MainPool,
    ) -> Result<String> {
        let user = db::main::user::queries::insert("tester", "", pool).await?;
        db::main::user::queries::set_roles(user.id, roles, pool).await?;
        if !geofence.is_empty() {
            db::main::user::queries::set_geofence(user.id, geofence, pool).await?;
        }
        let secret = "test-secret".to_string();
        db::main::access_token::queries::insert(
            user.id,
            String::new(),
            secret.clone(),
            roles.to_vec(),
            pool,
        )
        .await?;
        Ok(secret)
    }

    fn post_body(lat: f64, lon: f64, starts_at: &str) -> serde_json::Value {
        serde_json::json!({
            "lat": lat,
            "lon": lon,
            "name": "Bitcoin meetup",
            "website": "https://example.com",
            "starts_at": starts_at,
        })
    }

    #[test]
    async fn get_excludes_pending_events_by_default() -> Result<()> {
        let pool = pool();
        db::main::event::queries::insert_with_status(
            None,
            1.23,
            4.56,
            "pending".to_string(),
            "https://example.com".to_string(),
            datetime!(2999-01-01 0:00 UTC),
            None,
            crate::db::main::event::schema::Status::Pending,
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
        assert!(
            res.is_empty(),
            "pending events must not leak into the public snapshot"
        );
        Ok(())
    }

    #[test]
    async fn get_includes_pending_events_when_status_all_requested() -> Result<()> {
        let pool = pool();
        db::main::event::queries::insert_with_status(
            None,
            1.23,
            4.56,
            "pending".to_string(),
            "https://example.com".to_string(),
            datetime!(2999-01-01 0:00 UTC),
            None,
            crate::db::main::event::schema::Status::Pending,
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
        let req = TestRequest::get().uri("/?status=all").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!("pending", res[0]["status"].as_str().unwrap());
        Ok(())
    }

    #[test]
    async fn get_rejects_unknown_status() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?status=bogus").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[test]
    async fn post_requires_auth() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/events").service(super::post)),
        )
        .await;
        let req = TestRequest::post()
            .uri("/events")
            .set_json(post_body(1.0, 2.0, "2999-01-01T00:00:00Z"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn post_by_unprivileged_creates_pending() -> Result<()> {
        let pool = pool();
        let secret = seed_user_with_token(&[Role::User], &[], &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .service(scope("/events").service(super::post)),
        )
        .await;
        let req = TestRequest::post()
            .uri("/events")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .set_json(post_body(1.0, 2.0, "2999-01-01T00:00:00Z"))
            .to_request();
        let res: JsonObject = test::call_and_read_body_json(&app, req).await;
        assert_eq!("pending", res["status"].as_str().unwrap());
        assert!(res["submitted_by"]["id"].is_i64());
        assert_eq!("tester", res["submitted_by"]["name"].as_str().unwrap());
        let stored =
            db::main::event::queries::select_by_id(res["id"].as_i64().unwrap(), &pool).await?;
        assert_eq!(
            crate::db::main::event::schema::Status::Pending,
            stored.status
        );
        Ok(())
    }

    async fn seed_user_id_and_token(
        roles: &[Role],
        pool: &crate::db::main::MainPool,
    ) -> Result<(i64, String)> {
        let user = db::main::user::queries::insert("owner", "", pool).await?;
        db::main::user::queries::set_roles(user.id, roles, pool).await?;
        let secret = "owner-secret".to_string();
        db::main::access_token::queries::insert(
            user.id,
            String::new(),
            secret.clone(),
            roles.to_vec(),
            pool,
        )
        .await?;
        Ok((user.id, secret))
    }

    #[test]
    async fn get_me_requires_auth() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/users").service(super::get_me)),
        )
        .await;
        let req = TestRequest::get().uri("/users/me/events").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn get_me_lists_only_own_events_including_past_and_pending() -> Result<()> {
        let pool = pool();
        let (owner_id, secret) = seed_user_id_and_token(&[Role::User], &pool).await?;
        let pending = db::main::event::queries::insert_with_status(
            None,
            1.0,
            2.0,
            "mine pending".to_string(),
            "website".to_string(),
            datetime!(2999-01-01 0:00 UTC),
            None,
            crate::db::main::event::schema::Status::Pending,
            Some(owner_id),
            &pool,
        )
        .await?;
        let past_rejected = db::main::event::queries::insert_with_status(
            None,
            3.0,
            4.0,
            "mine past".to_string(),
            "website".to_string(),
            datetime!(2020-01-01 0:00 UTC),
            None,
            crate::db::main::event::schema::Status::Rejected,
            Some(owner_id),
            &pool,
        )
        .await?;
        db::main::event::queries::insert_with_status(
            None,
            5.0,
            6.0,
            "someone else".to_string(),
            "website".to_string(),
            datetime!(2999-01-01 0:00 UTC),
            None,
            crate::db::main::event::schema::Status::Live,
            Some(owner_id + 1),
            &pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/users").service(super::get_me)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/users/me/events")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;

        assert_eq!(2, res.len());
        let by_id: std::collections::HashMap<i64, &JsonObject> = res
            .iter()
            .map(|it| (it["id"].as_i64().unwrap(), it))
            .collect();
        assert_eq!("pending", by_id[&pending.id]["status"].as_str().unwrap());
        assert_eq!(
            owner_id,
            by_id[&pending.id]["submitted_by"]["id"].as_i64().unwrap()
        );
        assert_eq!(
            "owner",
            by_id[&pending.id]["submitted_by"]["name"].as_str().unwrap()
        );
        assert_eq!(
            "rejected",
            by_id[&past_rejected.id]["status"].as_str().unwrap()
        );
        Ok(())
    }

    #[test]
    async fn post_by_privileged_creates_live() -> Result<()> {
        let pool = pool();
        let secret = seed_user_with_token(&[Role::EventManager], &[], &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .service(scope("/events").service(super::post)),
        )
        .await;
        let req = TestRequest::post()
            .uri("/events")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .set_json(post_body(1.0, 2.0, "2999-01-01T00:00:00Z"))
            .to_request();
        let res: JsonObject = test::call_and_read_body_json(&app, req).await;
        assert_eq!("live", res["status"].as_str().unwrap());
        let stored =
            db::main::event::queries::select_by_id(res["id"].as_i64().unwrap(), &pool).await?;
        assert_eq!(crate::db::main::event::schema::Status::Live, stored.status);
        Ok(())
    }

    #[test]
    async fn post_follows_token_roles_over_user_roles() -> Result<()> {
        // A token with a non-empty role list is authoritative: it narrows or
        // changes the user's effective roles, exactly like the RPC layer.
        let pool = pool();

        // User is an event manager but the token is scoped down to `user`.
        let scoped_down = db::main::user::queries::insert("scoped_down", "", &pool).await?;
        db::main::user::queries::set_roles(scoped_down.id, &[Role::EventManager], &pool).await?;
        let down_secret = "down-secret".to_string();
        db::main::access_token::queries::insert(
            scoped_down.id,
            String::new(),
            down_secret.clone(),
            vec![Role::User],
            &pool,
        )
        .await?;

        // User is ordinary but the token carries the event-manager role.
        let scoped_up = db::main::user::queries::insert("scoped_up", "", &pool).await?;
        db::main::user::queries::set_roles(scoped_up.id, &[Role::User], &pool).await?;
        let up_secret = "up-secret".to_string();
        db::main::access_token::queries::insert(
            scoped_up.id,
            String::new(),
            up_secret.clone(),
            vec![Role::EventManager],
            &pool,
        )
        .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::post)),
        )
        .await;

        let req = TestRequest::post()
            .uri("/events")
            .insert_header((header::AUTHORIZATION, format!("Bearer {down_secret}")))
            .set_json(post_body(1.0, 2.0, "2999-01-01T00:00:00Z"))
            .to_request();
        let res: JsonObject = test::call_and_read_body_json(&app, req).await;
        assert_eq!(
            "pending",
            res["status"].as_str().unwrap(),
            "a down-scoped token must not inherit the user's event-manager role"
        );

        let req = TestRequest::post()
            .uri("/events")
            .insert_header((header::AUTHORIZATION, format!("Bearer {up_secret}")))
            .set_json(post_body(3.0, 4.0, "2999-01-01T00:00:00Z"))
            .to_request();
        let res: JsonObject = test::call_and_read_body_json(&app, req).await;
        assert_eq!(
            "live",
            res["status"].as_str().unwrap(),
            "a token with an elevated role grants privileges regardless of user roles"
        );
        Ok(())
    }

    #[test]
    async fn post_rejects_empty_name() -> Result<()> {
        let pool = pool();
        let secret = seed_user_with_token(&[Role::User], &[], &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::post)),
        )
        .await;
        let mut body = post_body(1.0, 2.0, "2999-01-01T00:00:00Z");
        body["name"] = serde_json::json!("   ");
        let req = TestRequest::post()
            .uri("/events")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .set_json(body)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[test]
    async fn post_rejects_out_of_range_lat() -> Result<()> {
        let pool = pool();
        let secret = seed_user_with_token(&[Role::User], &[], &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::post)),
        )
        .await;
        let req = TestRequest::post()
            .uri("/events")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .set_json(post_body(120.0, 2.0, "2999-01-01T00:00:00Z"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[test]
    async fn post_rejects_invalid_timestamp() -> Result<()> {
        let pool = pool();
        let secret = seed_user_with_token(&[Role::User], &[], &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::post)),
        )
        .await;
        let req = TestRequest::post()
            .uri("/events")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .set_json(post_body(1.0, 2.0, "not-a-date"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[test]
    async fn post_by_privileged_outside_geofence_is_forbidden() -> Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(phuket_area_tags(), &pool).await?;
        let secret = seed_user_with_token(&[Role::EventManager], &[area.id], &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::post)),
        )
        .await;
        // London, outside the Phuket fence.
        let req = TestRequest::post()
            .uri("/events")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .set_json(post_body(51.5, -0.1, "2999-01-01T00:00:00Z"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    #[test]
    async fn put_status_requires_auth() -> Result<()> {
        let pool = pool();
        let event = db::main::event::queries::insert(
            None,
            1.0,
            2.0,
            "name".to_string(),
            "website".to_string(),
            datetime!(2999-01-01 0:00 UTC),
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::put_status)),
        )
        .await;
        let req = TestRequest::put()
            .uri(&format!("/events/{}/status", event.id))
            .set_json(serde_json::json!({ "status": "rejected" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn put_status_is_forbidden_for_unprivileged() -> Result<()> {
        let pool = pool();
        let secret = seed_user_with_token(&[Role::User], &[], &pool).await?;
        let event = db::main::event::queries::insert(
            None,
            1.0,
            2.0,
            "name".to_string(),
            "website".to_string(),
            datetime!(2999-01-01 0:00 UTC),
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::put_status)),
        )
        .await;
        let req = TestRequest::put()
            .uri(&format!("/events/{}/status", event.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .set_json(serde_json::json!({ "status": "live" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    #[test]
    async fn put_status_admin_can_approve_pending() -> Result<()> {
        let pool = pool();
        let secret = seed_user_with_token(&[Role::Admin], &[], &pool).await?;
        let event = db::main::event::queries::insert_with_status(
            None,
            1.0,
            2.0,
            "name".to_string(),
            "website".to_string(),
            datetime!(2999-01-01 0:00 UTC),
            None,
            crate::db::main::event::schema::Status::Pending,
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .service(scope("/events").service(super::put_status)),
        )
        .await;
        let req = TestRequest::put()
            .uri(&format!("/events/{}/status", event.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .set_json(serde_json::json!({ "status": "live" }))
            .to_request();
        let res: JsonObject = test::call_and_read_body_json(&app, req).await;
        assert_eq!("live", res["status"].as_str().unwrap());
        let stored = db::main::event::queries::select_by_id(event.id, &pool).await?;
        assert_eq!(crate::db::main::event::schema::Status::Live, stored.status);
        Ok(())
    }

    #[test]
    async fn put_status_rejects_pending_target() -> Result<()> {
        let pool = pool();
        let secret = seed_user_with_token(&[Role::Admin], &[], &pool).await?;
        let event = db::main::event::queries::insert(
            None,
            1.0,
            2.0,
            "name".to_string(),
            "website".to_string(),
            datetime!(2999-01-01 0:00 UTC),
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::put_status)),
        )
        .await;
        let req = TestRequest::put()
            .uri(&format!("/events/{}/status", event.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .set_json(serde_json::json!({ "status": "pending" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[test]
    async fn put_status_not_found() -> Result<()> {
        let pool = pool();
        let secret = seed_user_with_token(&[Role::Root], &[], &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::put_status)),
        )
        .await;
        let req = TestRequest::put()
            .uri("/events/999/status")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .set_json(serde_json::json!({ "status": "rejected" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    #[test]
    async fn put_status_outside_geofence_is_forbidden() -> Result<()> {
        let pool = pool();
        let area = db::main::area::queries::insert(phuket_area_tags(), &pool).await?;
        let secret = seed_user_with_token(&[Role::EventManager], &[area.id], &pool).await?;
        // Event lives in London, reviewer is fenced to Phuket.
        let event = db::main::event::queries::insert_with_status(
            None,
            51.5,
            -0.1,
            "name".to_string(),
            "website".to_string(),
            datetime!(2999-01-01 0:00 UTC),
            None,
            crate::db::main::event::schema::Status::Pending,
            None,
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::put_status)),
        )
        .await;
        let req = TestRequest::put()
            .uri(&format!("/events/{}/status", event.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .set_json(serde_json::json!({ "status": "live" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    async fn seed_pending_event(owner_id: i64, pool: &crate::db::main::MainPool) -> Result<i64> {
        let event = db::main::event::queries::insert_with_status(
            None,
            1.0,
            2.0,
            "mine".to_string(),
            "website".to_string(),
            datetime!(2999-01-01 0:00 UTC),
            None,
            crate::db::main::event::schema::Status::Pending,
            Some(owner_id),
            pool,
        )
        .await?;
        Ok(event.id)
    }

    #[test]
    async fn delete_requires_auth() -> Result<()> {
        let pool = pool();
        let event_id = seed_pending_event(1, &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::delete)),
        )
        .await;
        let req = TestRequest::delete()
            .uri(&format!("/events/{event_id}"))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn delete_owner_revokes_own_pending() -> Result<()> {
        let pool = pool();
        let (owner_id, secret) = seed_user_id_and_token(&[Role::User], &pool).await?;
        let event_id = seed_pending_event(owner_id, &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .service(scope("/events").service(super::delete)),
        )
        .await;
        let req = TestRequest::delete()
            .uri(&format!("/events/{event_id}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .to_request();
        let res: JsonObject = test::call_and_read_body_json(&app, req).await;
        assert_eq!(event_id, res["id"].as_i64().unwrap());
        assert_eq!("pending", res["status"].as_str().unwrap());
        assert_eq!(
            owner_id,
            res["submitted_by"]["id"].as_i64().unwrap(),
            "the revoked event still carries its submitter"
        );
        let stored = db::main::event::queries::select_by_id(event_id, &pool).await?;
        assert!(stored.deleted_at.is_some());
        Ok(())
    }

    #[test]
    async fn delete_forbids_non_owner() -> Result<()> {
        let pool = pool();
        let (owner_id, _owner_secret) = seed_user_id_and_token(&[Role::User], &pool).await?;
        // A different signed-in user ("tester", secret "test-secret").
        let secret = seed_user_with_token(&[Role::User], &[], &pool).await?;
        let event_id = seed_pending_event(owner_id, &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .service(scope("/events").service(super::delete)),
        )
        .await;
        let req = TestRequest::delete()
            .uri(&format!("/events/{event_id}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        let stored = db::main::event::queries::select_by_id(event_id, &pool).await?;
        assert!(
            stored.deleted_at.is_none(),
            "another user's event must be left untouched"
        );
        Ok(())
    }

    #[test]
    async fn delete_forbids_own_live_event() -> Result<()> {
        let pool = pool();
        let (owner_id, secret) = seed_user_id_and_token(&[Role::User], &pool).await?;
        // Privileged submissions go straight to live; a regular owner still
        // must not be able to take a published event down through this route.
        let event = db::main::event::queries::insert_with_status(
            None,
            1.0,
            2.0,
            "published".to_string(),
            "website".to_string(),
            datetime!(2999-01-01 0:00 UTC),
            None,
            crate::db::main::event::schema::Status::Live,
            Some(owner_id),
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .service(scope("/events").service(super::delete)),
        )
        .await;
        let req = TestRequest::delete()
            .uri(&format!("/events/{}", event.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        let stored = db::main::event::queries::select_by_id(event.id, &pool).await?;
        assert!(stored.deleted_at.is_none());
        Ok(())
    }

    #[test]
    async fn delete_forbids_own_rejected_event() -> Result<()> {
        let pool = pool();
        let (owner_id, secret) = seed_user_id_and_token(&[Role::User], &pool).await?;
        let event = db::main::event::queries::insert_with_status(
            None,
            1.0,
            2.0,
            "rejected".to_string(),
            "website".to_string(),
            datetime!(2999-01-01 0:00 UTC),
            None,
            crate::db::main::event::schema::Status::Rejected,
            Some(owner_id),
            &pool,
        )
        .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::delete)),
        )
        .await;
        let req = TestRequest::delete()
            .uri(&format!("/events/{}", event.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    #[test]
    async fn delete_is_idempotent_for_owner() -> Result<()> {
        let pool = pool();
        let (owner_id, secret) = seed_user_id_and_token(&[Role::User], &pool).await?;
        let event_id = seed_pending_event(owner_id, &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::delete)),
        )
        .await;

        for _ in 0..2 {
            let req = TestRequest::delete()
                .uri(&format!("/events/{event_id}"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
                .to_request();
            let res = test::call_service(&app, req).await;
            assert_eq!(res.status(), StatusCode::OK);
        }
        Ok(())
    }

    #[test]
    async fn delete_not_found() -> Result<()> {
        let pool = pool();
        let secret = seed_user_with_token(&[Role::User], &[], &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::delete)),
        )
        .await;
        let req = TestRequest::delete()
            .uri("/events/999")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    #[test]
    async fn delete_hides_revoked_event_from_my_events() -> Result<()> {
        let pool = pool();
        let (owner_id, secret) = seed_user_id_and_token(&[Role::User], &pool).await?;
        let event_id = seed_pending_event(owner_id, &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::delete))
                .service(scope("/users").service(super::get_me)),
        )
        .await;

        let req = TestRequest::delete()
            .uri(&format!("/events/{event_id}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);

        let req = TestRequest::get()
            .uri("/users/me/events")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(
            res.is_empty(),
            "a revoked submission must disappear from the submitter's list"
        );
        Ok(())
    }

    #[test]
    async fn delete_appears_as_tombstone_in_delta() -> Result<()> {
        let pool = pool();
        let (owner_id, secret) = seed_user_id_and_token(&[Role::User], &pool).await?;
        let event_id = seed_pending_event(owner_id, &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/events").service(super::delete).service(super::get)),
        )
        .await;

        let req = TestRequest::delete()
            .uri(&format!("/events/{event_id}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);

        let req = TestRequest::get()
            .uri("/events?updated_since=1970-01-01T00:00:00Z&include_deleted=true&status=all")
            .to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(event_id, res[0]["id"].as_i64().unwrap());
        assert!(
            res[0]["deleted_at"].is_string(),
            "the revocation must be observable as a tombstone"
        );
        Ok(())
    }
}
