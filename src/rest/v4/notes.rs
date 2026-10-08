use crate::db;
use crate::db::main::note::schema::Note;
use crate::db::main::MainPool;
use crate::rest::auth::Auth;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult;
use crate::Error;
use actix_web::delete;
use actix_web::get;
use actix_web::patch;
use actix_web::post;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use time::OffsetDateTime;

const MAX_TEXT_LEN: usize = 2000;
const DEFAULT_RADIUS_KM: f64 = 10.0;
const MAX_RADIUS_KM: f64 = 100.0;
const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 500;

#[derive(Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, rename = "NoteAuthor")]
pub struct Author {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
}

/// A note as returned to its owner (includes private ones) and, for public
/// notes, to everyone else.
#[derive(Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, rename = "Note")]
pub struct Item {
    #[ts(type = "number")]
    pub id: i64,
    pub lat: f64,
    pub lon: f64,
    pub text: String,
    pub public: bool,
    pub author: Author,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub updated_at: OffsetDateTime,
    /// Only present on owner responses that asked for `include_deleted=true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    #[ts(optional, type = "string")]
    pub deleted_at: Option<OffsetDateTime>,
    /// Only present on search responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub distance_km: Option<f64>,
}

impl Item {
    fn new(note: Note, author: Author) -> Self {
        Item {
            id: note.id,
            lat: note.lat,
            lon: note.lon,
            text: note.text,
            public: note.public,
            author,
            created_at: note.created_at,
            updated_at: note.updated_at,
            deleted_at: note.deleted_at,
            distance_km: None,
        }
    }
}

fn validate_coords(lat: f64, lon: f64) -> Result<(), RestApiError> {
    if !lat.is_finite()
        || !lon.is_finite()
        || !(-90.0..=90.0).contains(&lat)
        || !(-180.0..=180.0).contains(&lon)
    {
        return Err(RestApiError::invalid_input(
            "lat must be within [-90, 90] and lon within [-180, 180]",
        ));
    }
    Ok(())
}

fn validate_text(text: &str) -> Result<String, RestApiError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(RestApiError::invalid_input("Note text cannot be empty"));
    }
    if text.chars().count() > MAX_TEXT_LEN {
        return Err(RestApiError::invalid_input(format!(
            "Note text cannot exceed {MAX_TEXT_LEN} characters"
        )));
    }
    Ok(text.to_string())
}

fn not_found_from(err: Error) -> RestApiError {
    match err {
        Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
        _ => RestApiError::database(),
    }
}

/// Resolve `{id, name}` authors for a batch of notes.
async fn resolve_authors(
    notes: &[Note],
    pool: &MainPool,
) -> Result<std::collections::HashMap<i64, Author>, RestApiError> {
    let ids: Vec<i64> = notes
        .iter()
        .map(|note| note.user_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let users = db::main::user::queries::select_by_ids(&ids, pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(users
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

/// Fetch + attach authors for a set of notes. Users missing from the database
/// (should not happen, `user_id` is a foreign key) are dropped.
async fn attach_authors(notes: Vec<Note>, pool: &MainPool) -> Result<Vec<Item>, RestApiError> {
    let authors = resolve_authors(&notes, pool).await?;
    Ok(notes
        .into_iter()
        .filter_map(|note| {
            authors
                .get(&note.user_id)
                .cloned()
                .map(|a| Item::new(note, a))
        })
        .collect())
}

/// Fetch a single note, mapping a missing row to `404`.
async fn fetch(id: i64, pool: &MainPool) -> Result<Note, RestApiError> {
    db::main::note::queries::select_by_id(id, pool)
        .await
        .map_err(not_found_from)
}

#[derive(Deserialize)]
pub struct MeArgs {
    #[serde(default = "default_updated_since")]
    #[serde(with = "time::serde::rfc3339")]
    updated_since: OffsetDateTime,
    #[serde(default)]
    include_deleted: bool,
    #[serde(default = "default_limit")]
    limit: i64,
}

const fn default_updated_since() -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH
}

const fn default_limit() -> i64 {
    DEFAULT_LIMIT
}

/// `GET /v4/users/me/notes`
///
/// Lists every note belonging to the authenticated user, public and private.
/// Supports `updated_since`/`include_deleted` so a client can cache and
/// incrementally refresh its own notes.
#[get("/me/notes")]
pub async fn get_me(
    auth: Auth,
    args: Query<MeArgs>,
    pool: Data<MainPool>,
) -> RestResult<Vec<Item>> {
    let user = auth.user.ok_or_else(RestApiError::unauthorized)?;
    let limit = args.limit.clamp(1, MAX_LIMIT);
    let notes = db::main::note::queries::select_by_user_id(
        user.id,
        args.updated_since,
        args.include_deleted,
        limit,
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;

    let author = Author {
        id: user.id,
        name: user.name.clone(),
    };
    Ok(Json(
        notes
            .into_iter()
            .map(|note| Item::new(note, author.clone()))
            .collect(),
    ))
}

#[derive(Deserialize, ts_rs::TS)]
#[ts(export, rename = "PostNoteArgs")]
pub struct PostArgs {
    pub lat: f64,
    pub lon: f64,
    pub text: String,
    #[serde(default)]
    pub public: bool,
}

/// `POST /v4/notes`
///
/// Creates a note owned by the caller. Notes are private unless `public` is set.
#[post("")]
pub async fn post(auth: Auth, args: Json<PostArgs>, pool: Data<MainPool>) -> RestResult<Item> {
    let user = auth.user.ok_or_else(RestApiError::unauthorized)?;
    validate_coords(args.lat, args.lon)?;
    let text = validate_text(&args.text)?;

    let note =
        db::main::note::queries::insert(user.id, args.lat, args.lon, text, args.public, &pool)
            .await
            .map_err(|_| RestApiError::database())?;
    Ok(Json(Item::new(
        note,
        Author {
            id: user.id,
            name: user.name.clone(),
        },
    )))
}

/// `GET /v4/notes/{id}`
///
/// Public notes are readable by anyone. Private notes are only readable by
/// their owner; everyone else gets `404` so their existence is not leaked.
#[get("/{id}")]
pub async fn get_by_id(auth: Auth, path: Path<i64>, pool: Data<MainPool>) -> RestResult<Item> {
    let note = fetch(path.into_inner(), &pool).await?;
    if note.deleted_at.is_some() {
        return Err(RestApiError::not_found());
    }
    let is_owner = auth.user.as_ref().map(|user| user.id) == Some(note.user_id);
    if !note.public && !is_owner {
        return Err(RestApiError::not_found());
    }
    attach_authors(vec![note], &pool)
        .await?
        .pop()
        .map(Json)
        .ok_or_else(RestApiError::database)
}

#[derive(Deserialize, ts_rs::TS)]
#[ts(export, rename = "PatchNoteArgs")]
pub struct PatchArgs {
    #[serde(default)]
    #[ts(optional)]
    pub text: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub public: Option<bool>,
}

/// `PATCH /v4/notes/{id}`
///
/// Edits the caller's own note. Either `text`, `public` or both may be sent;
/// visibility can be flipped at any time.
#[patch("/{id}")]
pub async fn patch(
    auth: Auth,
    path: Path<i64>,
    args: Json<PatchArgs>,
    pool: Data<MainPool>,
) -> RestResult<Item> {
    let user = auth.user.ok_or_else(RestApiError::unauthorized)?;
    if args.text.is_none() && args.public.is_none() {
        return Err(RestApiError::invalid_input(
            "Provide text and/or public to update",
        ));
    }

    let id = path.into_inner();
    let existing = fetch(id, &pool).await?;
    if existing.deleted_at.is_some() || existing.user_id != user.id {
        return Err(RestApiError::not_found());
    }

    let text = match &args.text {
        Some(text) => validate_text(text)?,
        None => existing.text.clone(),
    };
    let public = args.public.unwrap_or(existing.public);

    let note = db::main::note::queries::update(id, text, public, &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(Json(Item::new(
        note,
        Author {
            id: user.id,
            name: user.name.clone(),
        },
    )))
}

/// `DELETE /v4/notes/{id}`
///
/// Soft-deletes the caller's own note.
#[delete("/{id}")]
pub async fn delete(auth: Auth, path: Path<i64>, pool: Data<MainPool>) -> RestResult<Item> {
    let user = auth.user.ok_or_else(RestApiError::unauthorized)?;
    let id = path.into_inner();
    let existing = fetch(id, &pool).await?;
    if existing.deleted_at.is_some() || existing.user_id != user.id {
        return Err(RestApiError::not_found());
    }

    let note = db::main::note::queries::set_deleted_at(id, Some(OffsetDateTime::now_utc()), &pool)
        .await
        .map_err(|_| RestApiError::database())?;
    Ok(Json(Item::new(
        note,
        Author {
            id: user.id,
            name: user.name.clone(),
        },
    )))
}

#[derive(Deserialize)]
pub struct SearchArgs {
    pub lat: f64,
    pub lon: f64,
    #[serde(default = "default_radius_km")]
    pub radius_km: f64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

const fn default_radius_km() -> f64 {
    DEFAULT_RADIUS_KM
}

/// Equirectangular distance in kilometers between two points. Good enough for
/// the small radii notes are searched with.
fn distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * a.sqrt().asin()
}

/// `GET /v4/notes/search`
///
/// Returns public notes around a point, nearest last. Private notes are never
/// returned. Intended for map clients loading notes in the current viewport.
#[get("/search")]
pub async fn search(args: Query<SearchArgs>, pool: Data<MainPool>) -> RestResult<Vec<Item>> {
    validate_coords(args.lat, args.lon)?;
    if !args.radius_km.is_finite() || args.radius_km <= 0.0 {
        return Err(RestApiError::invalid_input(
            "radius_km must be a positive number",
        ));
    }
    let radius_km = args.radius_km.min(MAX_RADIUS_KM);
    let limit = args.limit.clamp(1, MAX_LIMIT);

    let lat_delta = radius_km / 111.0;
    let cos_lat = args.lat.to_radians().cos().abs().max(1e-6);
    let lon_delta = radius_km / (111.0 * cos_lat);
    let min_lat = (args.lat - lat_delta).max(-90.0);
    let max_lat = (args.lat + lat_delta).min(90.0);
    let min_lon = (args.lon - lon_delta).max(-180.0);
    let max_lon = (args.lon + lon_delta).min(180.0);

    let notes = db::main::note::queries::select_public_in_bbox(
        min_lat, max_lat, min_lon, max_lon, limit, &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;

    let mut items = attach_authors(notes, &pool).await?;
    for item in &mut items {
        item.distance_km = Some(distance_km(args.lat, args.lon, item.lat, item.lon));
    }
    items.sort_by(|a, b| {
        a.distance_km
            .unwrap_or(f64::MAX)
            .total_cmp(&b.distance_km.unwrap_or(f64::MAX))
    });
    Ok(Json(items))
}

#[cfg(test)]
mod test {
    use crate::db::main::test::pool;
    use crate::db::main::user::schema::Role;
    use crate::{db, Result};
    use actix_web::http::header;
    use actix_web::http::header::ContentType;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};

    async fn seed_user_with_token(pool: &crate::db::main::MainPool) -> Result<(i64, String)> {
        let user = db::main::user::queries::insert("tester", "", pool).await?;
        let secret = "test-secret".to_string();
        db::main::access_token::queries::insert(
            user.id,
            String::new(),
            secret.clone(),
            vec![Role::User],
            pool,
        )
        .await?;
        Ok((user.id, secret))
    }

    macro_rules! app {
        ($pool:expr) => {
            test::init_service(
                App::new()
                    .app_data(Data::new($pool))
                    .service(
                        scope("/notes")
                            .service(super::search)
                            .service(super::post)
                            .service(super::get_by_id)
                            .service(super::patch)
                            .service(super::delete),
                    )
                    .service(scope("/users").service(super::get_me)),
            )
            .await
        };
    }

    #[test]
    async fn post_requires_auth() -> Result<()> {
        let app = app!(pool());
        let req = TestRequest::post()
            .uri("/notes")
            .insert_header(ContentType::json())
            .set_payload(r#"{"lat":1.0,"lon":2.0,"text":"hi"}"#.as_bytes())
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn post_creates_a_private_note_by_default() -> Result<()> {
        let pool = pool();
        let (user_id, secret) = seed_user_with_token(&pool).await?;
        let app = app!(pool.clone());

        let req = TestRequest::post()
            .uri("/notes")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"lat":1.5,"lon":2.5,"text":"  hello  "}"#.as_bytes())
            .to_request();
        let res: super::Item = test::call_and_read_body_json(&app, req).await;

        assert_eq!(res.text, "hello");
        assert!(!res.public);
        assert_eq!(res.author.id, user_id);
        assert_eq!(res.author.name, "tester");

        let stored = db::main::note::queries::select_by_id(res.id, &pool).await?;
        assert_eq!(stored.user_id, user_id);
        assert!(!stored.public);
        Ok(())
    }

    #[test]
    async fn post_rejects_empty_text() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;
        let app = app!(pool);
        let req = TestRequest::post()
            .uri("/notes")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"lat":1.0,"lon":2.0,"text":"   "}"#.as_bytes())
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[test]
    async fn post_rejects_invalid_coordinates() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;
        let app = app!(pool);
        let req = TestRequest::post()
            .uri("/notes")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"lat":91.0,"lon":2.0,"text":"hi"}"#.as_bytes())
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[test]
    async fn get_me_returns_all_own_notes() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;
        db::main::note::queries::insert(1, 1.0, 1.0, "private", false, &pool).await?;
        db::main::note::queries::insert(1, 2.0, 2.0, "public", true, &pool).await?;
        db::main::note::queries::insert(2, 3.0, 3.0, "other", true, &pool).await?;
        let app = app!(pool);

        let req = TestRequest::get()
            .uri("/users/me/notes")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .to_request();
        let res: Vec<super::Item> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 2);
        Ok(())
    }

    #[test]
    async fn get_me_requires_auth() -> Result<()> {
        let app = app!(pool());
        let res =
            test::call_service(&app, TestRequest::get().uri("/users/me/notes").to_request()).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[test]
    async fn search_returns_public_notes_with_author_and_hides_private() -> Result<()> {
        let pool = pool();
        db::main::user::queries::insert("alice", "", &pool).await?;
        db::main::note::queries::insert(1, 53.5, 9.9, "public", true, &pool).await?;
        db::main::note::queries::insert(1, 53.5, 9.9, "private", false, &pool).await?;
        db::main::note::queries::insert(1, 40.0, 40.0, "far", true, &pool).await?;
        let app = app!(pool);

        let req = TestRequest::get()
            .uri("/notes/search?lat=53.5&lon=9.9&radius_km=5")
            .to_request();
        let res: Vec<super::Item> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].text, "public");
        assert_eq!(res[0].author.name, "alice");
        assert!(res[0].distance_km.is_some());
        Ok(())
    }

    #[test]
    async fn search_rejects_non_positive_radius() -> Result<()> {
        let app = app!(pool());
        let res = test::call_service(
            &app,
            TestRequest::get()
                .uri("/notes/search?lat=1&lon=1&radius_km=0")
                .to_request(),
        )
        .await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[test]
    async fn get_by_id_hides_private_note_from_others() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;
        let note = db::main::note::queries::insert(1, 1.0, 1.0, "private", false, &pool).await?;
        let app = app!(pool);

        let anon = test::call_service(
            &app,
            TestRequest::get()
                .uri(&format!("/notes/{}", note.id))
                .to_request(),
        )
        .await;
        assert_eq!(anon.status(), StatusCode::NOT_FOUND);

        let owner = test::call_service(
            &app,
            TestRequest::get()
                .uri(&format!("/notes/{}", note.id))
                .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
                .to_request(),
        )
        .await;
        assert_eq!(owner.status(), StatusCode::OK);
        Ok(())
    }

    #[test]
    async fn patch_toggles_visibility_and_text() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;
        let note = db::main::note::queries::insert(1, 1.0, 1.0, "old", false, &pool).await?;
        let app = app!(pool.clone());

        let req = TestRequest::patch()
            .uri(&format!("/notes/{}", note.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"text":"new","public":true}"#.as_bytes())
            .to_request();
        let res: super::Item = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.text, "new");
        assert!(res.public);

        let stored = db::main::note::queries::select_by_id(note.id, &pool).await?;
        assert!(stored.public);
        assert_eq!(stored.text, "new");
        Ok(())
    }

    #[test]
    async fn patch_rejects_note_owned_by_someone_else() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;
        let note = db::main::note::queries::insert(2, 1.0, 1.0, "theirs", false, &pool).await?;
        let app = app!(pool);

        let req = TestRequest::patch()
            .uri(&format!("/notes/{}", note.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"public":true}"#.as_bytes())
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    #[test]
    async fn delete_soft_deletes_own_note() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;
        let note = db::main::note::queries::insert(1, 1.0, 1.0, "bye", true, &pool).await?;
        let app = app!(pool.clone());

        let req = TestRequest::delete()
            .uri(&format!("/notes/{}", note.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);

        let stored = db::main::note::queries::select_by_id(note.id, &pool).await?;
        assert!(stored.deleted_at.is_some());
        Ok(())
    }

    #[test]
    async fn patch_makes_note_private_again() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;
        let note = db::main::note::queries::insert(1, 1.0, 1.0, "public", true, &pool).await?;
        let app = app!(pool.clone());

        let req = TestRequest::patch()
            .uri(&format!("/notes/{}", note.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{"public":false}"#.as_bytes())
            .to_request();
        let res: super::Item = test::call_and_read_body_json(&app, req).await;
        assert!(!res.public);

        let stored = db::main::note::queries::select_by_id(note.id, &pool).await?;
        assert!(!stored.public);
        Ok(())
    }

    #[test]
    async fn patch_rejects_request_without_changes() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;
        let note = db::main::note::queries::insert(1, 1.0, 1.0, "note", false, &pool).await?;
        let app = app!(pool);

        let req = TestRequest::patch()
            .uri(&format!("/notes/{}", note.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(r#"{}"#.as_bytes())
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[test]
    async fn get_by_id_returns_404_for_deleted_note() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;
        let note = db::main::note::queries::insert(1, 1.0, 1.0, "gone", true, &pool).await?;
        db::main::note::queries::set_deleted_at(
            note.id,
            Some(time::OffsetDateTime::now_utc()),
            &pool,
        )
        .await?;
        let app = app!(pool);

        let req = TestRequest::get()
            .uri(&format!("/notes/{}", note.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    #[test]
    async fn delete_rejects_note_owned_by_someone_else() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;
        let note = db::main::note::queries::insert(2, 1.0, 1.0, "theirs", true, &pool).await?;
        let app = app!(pool.clone());

        let req = TestRequest::delete()
            .uri(&format!("/notes/{}", note.id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        let stored = db::main::note::queries::select_by_id(note.id, &pool).await?;
        assert!(stored.deleted_at.is_none());
        Ok(())
    }

    #[test]
    async fn search_respects_limit() -> Result<()> {
        let pool = pool();
        db::main::user::queries::insert("alice", "", &pool).await?;
        db::main::note::queries::insert(1, 53.5, 9.9, "one", true, &pool).await?;
        db::main::note::queries::insert(1, 53.5, 9.9, "two", true, &pool).await?;
        db::main::note::queries::insert(1, 53.5, 9.9, "three", true, &pool).await?;
        let app = app!(pool);

        let req = TestRequest::get()
            .uri("/notes/search?lat=53.5&lon=9.9&radius_km=5&limit=2")
            .to_request();
        let res: Vec<super::Item> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 2);
        Ok(())
    }

    #[test]
    async fn search_rejects_invalid_coordinates() -> Result<()> {
        let app = app!(pool());
        let res = test::call_service(
            &app,
            TestRequest::get()
                .uri("/notes/search?lat=999&lon=1")
                .to_request(),
        )
        .await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[test]
    async fn post_rejects_oversized_text() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;
        let app = app!(pool);
        let payload = format!(
            r#"{{"lat":1.0,"lon":2.0,"text":"{}"}}"#,
            "a".repeat(super::MAX_TEXT_LEN + 1)
        );
        let req = TestRequest::post()
            .uri("/notes")
            .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
            .insert_header(ContentType::json())
            .set_payload(payload)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[test]
    async fn get_me_updated_since_filters_by_cursor() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;
        db::main::note::queries::insert(1, 1.0, 1.0, "one", false, &pool).await?;
        db::main::note::queries::insert(1, 2.0, 2.0, "two", true, &pool).await?;
        let app = app!(pool);

        let all: Vec<super::Item> = test::call_and_read_body_json(
            &app,
            TestRequest::get()
                .uri("/users/me/notes")
                .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
                .to_request(),
        )
        .await;
        assert_eq!(all.len(), 2);

        let future: Vec<super::Item> = test::call_and_read_body_json(
            &app,
            TestRequest::get()
                .uri("/users/me/notes?updated_since=2100-01-01T00:00:00Z")
                .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
                .to_request(),
        )
        .await;
        assert!(future.is_empty());
        Ok(())
    }

    #[test]
    async fn get_me_excludes_deleted_unless_requested() -> Result<()> {
        let pool = pool();
        let (_, secret) = seed_user_with_token(&pool).await?;
        let note = db::main::note::queries::insert(1, 1.0, 1.0, "gone", true, &pool).await?;
        db::main::note::queries::set_deleted_at(
            note.id,
            Some(time::OffsetDateTime::now_utc()),
            &pool,
        )
        .await?;
        let app = app!(pool);

        let live: Vec<super::Item> = test::call_and_read_body_json(
            &app,
            TestRequest::get()
                .uri("/users/me/notes")
                .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
                .to_request(),
        )
        .await;
        assert!(live.is_empty());

        let with_deleted: Vec<super::Item> = test::call_and_read_body_json(
            &app,
            TestRequest::get()
                .uri("/users/me/notes?include_deleted=true")
                .insert_header((header::AUTHORIZATION, format!("Bearer {secret}")))
                .to_request(),
        )
        .await;
        assert_eq!(with_deleted.len(), 1);
        assert!(with_deleted[0].deleted_at.is_some());
        Ok(())
    }
}
