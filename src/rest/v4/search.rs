use crate::db;
use crate::db::main::area::queries::RankedArea;
use crate::db::main::element::queries::RankedElement;
use crate::db::main::MainPool;
use crate::rest::error::RestResult as Res;
use crate::rest::error::{RestApiError, RestApiErrorCode};
use crate::rest::v4::places::SearchedPlace;
use actix_web::{get, web::Data, web::Json, web::Query};
use serde::{Deserialize, Serialize};

const MIN_QUERY_LEN: usize = 3;
const MAX_LIMIT: i64 = 100;
const MAX_OFFSET: i64 = 10_000;

#[derive(Deserialize)]
pub struct SearchArgs {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// `area` or `place`. Omit for both.
    pub type_filter: Option<String>,
}

fn default_limit() -> i64 {
    20
}

/// Whether a reachable next page exists. `offset` is clamped to `MAX_OFFSET`, so
/// the next page (`offset + limit`) is only reachable when it, too, is within the
/// cap. Without the cap check, `has_more` stays `true` past the cap while the
/// clamp keeps returning the same page — an infinite pagination loop.
fn has_next_page(offset: i64, limit: i64, total: i64) -> bool {
    let next_offset = offset + limit;
    next_offset < total && next_offset <= MAX_OFFSET
}

#[derive(Serialize)]
pub struct SearchedArea {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    /// `[west, south, east, north]`. Absent when the area has no bbox of its own.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
}

/// `SearchedPlace` is boxed because it is an order of magnitude larger than
/// `SearchedArea`, and clippy's `large_enum_variant` would otherwise fire.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SearchResult {
    Area(SearchedArea),
    Place(Box<SearchedPlace>),
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total_count: u32,
    pub has_more: bool,
    pub query: String,
    pub pagination: PaginationInfo,
}

#[derive(Serialize)]
pub struct PaginationInfo {
    pub offset: i64,
    pub limit: i64,
    pub total: u32,
}

/// One candidate row plus its global sort key. Areas carry `kind = 0` so they
/// precede places at equal rank; `distance` only ever applies to places. `id` is
/// the final, unique tiebreaker so the merged order is total and identical
/// between the independent SQL runs that serve consecutive pages — without it,
/// rows tied on every other key can shuffle and be skipped or duplicated across
/// page boundaries. It mirrors the `id` term each side's SQL `ORDER BY` ends on.
struct Ranked {
    rank: i64,
    kind: u8,
    distance: f64,
    name_len: usize,
    name: String,
    id: i64,
    result: SearchResult,
}

// GET /v4/search?q=hamburg&lat=53.5&lon=9.9&limit=20&offset=0&type_filter=place
#[get("")]
pub async fn get(args: Query<SearchArgs>, pool: Data<MainPool>) -> Res<SearchResponse> {
    let query = args.q.trim().to_string();
    if query.chars().count() < MIN_QUERY_LEN {
        return Err(RestApiError::new(
            RestApiErrorCode::InvalidInput,
            "Search query must be at least 3 characters long",
        ));
    }

    let (want_area, want_place) = match args.type_filter.as_deref() {
        None => (true, true),
        Some("area") => (true, false),
        Some("place") => (false, true),
        Some(_) => {
            return Err(RestApiError::new(
                RestApiErrorCode::InvalidInput,
                "type_filter must be 'area' or 'place'",
            ))
        }
    };

    let location = match (args.lat, args.lon) {
        (Some(lat), Some(lon)) => Some((lat, lon)),
        (None, None) => None,
        _ => {
            return Err(RestApiError::new(
                RestApiErrorCode::InvalidInput,
                "lat and lon must be provided together",
            ))
        }
    };

    let limit = args.limit.clamp(1, MAX_LIMIT);
    let offset = args.offset.clamp(0, MAX_OFFSET);
    // Each side returns its own top `offset + limit`. Merging two lists sorted by
    // the same key and re-slicing yields exactly the global page.
    let row_limit = offset + limit;

    let mut ranked: Vec<Ranked> = Vec::new();
    let mut total: i64 = 0;

    if want_area {
        let areas = db::main::area::queries::select_by_search(query.clone(), row_limit, &pool)
            .await
            .map_err(|_| RestApiError::database())?;
        total += db::main::area::queries::count_by_search(query.clone(), &pool)
            .await
            .map_err(|_| RestApiError::database())?;
        for area in areas {
            let RankedArea {
                id,
                name,
                alias,
                bbox,
                rank,
            } = area;
            ranked.push(Ranked {
                rank,
                kind: 0,
                distance: 0.0,
                name_len: name.chars().count(),
                name: name.clone(),
                id,
                result: SearchResult::Area(SearchedArea {
                    id,
                    name,
                    alias,
                    bbox,
                }),
            });
        }
    }

    if want_place {
        let elements = db::main::element::queries::select_by_tag_value_search(
            query.clone(),
            location,
            row_limit,
            &pool,
        )
        .await
        .map_err(|_| RestApiError::database())?;
        total += db::main::element::queries::count_by_tag_value_search(query.clone(), &pool)
            .await
            .map_err(|_| RestApiError::database())?;
        for ranked_element in elements {
            let RankedElement { element, rank } = ranked_element;
            // The query filters NULL coordinates, so these defaults never apply.
            let distance = match location {
                Some((lat, lon)) => {
                    let place_lat = element.lat.unwrap_or(0.0);
                    let place_lon = element.lon.unwrap_or(0.0);
                    (place_lat - lat).powi(2) + (place_lon - lon).powi(2)
                }
                None => 0.0,
            };
            let place: SearchedPlace = element.into();
            let name = place.name.clone();
            let id = place.id;
            ranked.push(Ranked {
                rank,
                kind: 1,
                distance,
                name_len: name.chars().count(),
                name,
                id,
                result: SearchResult::Place(Box::new(place)),
            });
        }
    }

    ranked.sort_by(|a, b| {
        a.rank
            .cmp(&b.rank)
            .then(a.kind.cmp(&b.kind))
            .then(a.distance.total_cmp(&b.distance))
            .then(a.name_len.cmp(&b.name_len))
            .then(a.name.cmp(&b.name))
            .then(a.id.cmp(&b.id))
    });

    let results: Vec<SearchResult> = ranked
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(|it| it.result)
        .collect();

    let total = total.max(0) as u32;

    Ok(Json(SearchResponse {
        results,
        total_count: total,
        has_more: has_next_page(offset, limit, total as i64),
        query,
        pagination: PaginationInfo {
            offset,
            limit,
            total,
        },
    }))
}

#[cfg(test)]
mod test {
    use super::{has_next_page, MAX_OFFSET};
    use crate::db;
    use crate::db::main::test::pool;
    use crate::db::main::MainPool;
    use crate::service::overpass::OverpassElement;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use serde_json::{json, Map, Value};

    #[test]
    async fn has_next_page_stops_at_the_offset_cap() {
        // Plenty of rows remain, but the next page would be past the cap, where
        // the clamp would just re-serve the current page. Must report no more.
        assert!(!has_next_page(MAX_OFFSET, 20, 25_000));
        assert!(!has_next_page(MAX_OFFSET - 10, 20, 25_000));
        // Next page lands exactly on the cap — still reachable.
        assert!(has_next_page(MAX_OFFSET - 20, 20, 25_000));
        // Ordinary cases well within the cap.
        assert!(has_next_page(0, 20, 25_000));
        assert!(!has_next_page(0, 20, 15));
    }

    macro_rules! app {
        ($pool:expr) => {
            test::init_service(
                App::new()
                    .app_data(Data::new($pool))
                    .service(scope("/search").service(super::get)),
            )
            .await
        };
    }

    async fn insert_place(id: i64, tags: &[(&str, &str)], lat: f64, lon: f64, pool: &MainPool) {
        let element =
            db::main::element::queries::insert(OverpassElement::mock_with_tags(id, tags), pool)
                .await
                .unwrap();
        db::main::element::queries::set_lat_lon(element.id, lat, lon, pool)
            .await
            .unwrap();
    }

    async fn insert_area(name: &str, alias: &str, pool: &MainPool) {
        let mut tags = Map::new();
        tags.insert("name".into(), Value::String(name.into()));
        tags.insert("url_alias".into(), Value::String(alias.into()));
        tags.insert(
            "geo_json".into(),
            json!({"type":"Feature","properties":{},"geometry":{"type":"Point","coordinates":[9.99,53.55]}}),
        );
        db::main::area::queries::insert(tags, pool).await.unwrap();
    }

    async fn status(uri: &str, pool: MainPool) -> u16 {
        let app = app!(pool);
        let res = test::call_service(&app, TestRequest::get().uri(uri).to_request()).await;
        res.status().as_u16()
    }

    #[test]
    async fn rejects_short_query() -> Result<()> {
        assert_eq!(400, status("/search?q=ab", pool()).await);
        Ok(())
    }

    #[test]
    async fn rejects_empty_query() -> Result<()> {
        assert_eq!(400, status("/search?q=", pool()).await);
        Ok(())
    }

    #[test]
    async fn rejects_unknown_type_filter() -> Result<()> {
        assert_eq!(
            400,
            status("/search?q=hamburg&type_filter=element", pool()).await
        );
        Ok(())
    }

    #[test]
    async fn rejects_lat_without_lon() -> Result<()> {
        assert_eq!(400, status("/search?q=hamburg&lat=53.5", pool()).await);
        Ok(())
    }

    #[test]
    async fn finds_places_by_address() -> Result<()> {
        let pool = pool();
        insert_place(
            1,
            &[("name", "Kaffeeklatsch"), ("addr:city", "Hamburg")],
            53.5,
            9.9,
            &pool,
        )
        .await;
        insert_place(
            2,
            &[("name", "Nordsee"), ("addr:city", "Berlin")],
            52.5,
            13.4,
            &pool,
        )
        .await;
        let app = app!(pool);
        let req = TestRequest::get()
            .uri("/search?q=hamburg&type_filter=place")
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res["results"].as_array().unwrap().len());
        assert_eq!("place", res["results"][0]["type"]);
        assert_eq!("Kaffeeklatsch", res["results"][0]["name"]);
        assert_eq!(1, res["total_count"]);
        Ok(())
    }

    #[test]
    async fn place_rows_carry_coordinates_and_icon() -> Result<()> {
        let pool = pool();
        insert_place(
            1,
            &[("name", "Kaffeeklatsch"), ("addr:city", "Hamburg")],
            53.5,
            9.9,
            &pool,
        )
        .await;
        let app = app!(pool);
        let req = TestRequest::get()
            .uri("/search?q=hamburg&type_filter=place")
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        let row = &res["results"][0];
        assert_eq!(53.5, row["lat"]);
        assert_eq!(9.9, row["lon"]);
        assert!(row["icon"].is_string());
        Ok(())
    }

    #[test]
    async fn areas_precede_places_at_equal_rank() -> Result<()> {
        let pool = pool();
        insert_area("Hamburg", "hamburg", &pool).await;
        insert_place(1, &[("name", "Hamburg")], 53.5, 9.9, &pool).await;
        let app = app!(pool);
        let req = TestRequest::get().uri("/search?q=hamburg").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!("area", res["results"][0]["type"]);
        assert_eq!("place", res["results"][1]["type"]);
        assert_eq!(2, res["total_count"]);
        Ok(())
    }

    #[test]
    async fn area_rows_carry_alias() -> Result<()> {
        let pool = pool();
        insert_area("Hamburg", "hamburg", &pool).await;
        let app = app!(pool);
        let req = TestRequest::get()
            .uri("/search?q=hamburg&type_filter=area")
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!("hamburg", res["results"][0]["alias"]);
        Ok(())
    }

    #[test]
    async fn type_filter_area_excludes_places() -> Result<()> {
        let pool = pool();
        insert_area("Hamburg", "hamburg", &pool).await;
        insert_place(
            1,
            &[("name", "Kaffeeklatsch"), ("addr:city", "Hamburg")],
            53.5,
            9.9,
            &pool,
        )
        .await;
        let app = app!(pool);
        let req = TestRequest::get()
            .uri("/search?q=hamburg&type_filter=area")
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        for row in res["results"].as_array().unwrap() {
            assert_eq!("area", row["type"]);
        }
        assert_eq!(1, res["total_count"]);
        Ok(())
    }

    #[test]
    async fn paginates_over_a_stable_order() -> Result<()> {
        let pool = pool();
        for id in 1..=5 {
            insert_place(
                id,
                &[
                    ("name", "Hamburg"),
                    ("addr:street", &format!("Street {id}")),
                ],
                53.5,
                9.9,
                &pool,
            )
            .await;
        }
        let app = app!(pool);

        let req = TestRequest::get()
            .uri("/search?q=hamburg&limit=2&offset=0")
            .to_request();
        let first: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, first["results"].as_array().unwrap().len());
        assert_eq!(5, first["total_count"]);
        assert_eq!(true, first["has_more"]);

        let req = TestRequest::get()
            .uri("/search?q=hamburg&limit=2&offset=4")
            .to_request();
        let last: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, last["results"].as_array().unwrap().len());
        assert_eq!(false, last["has_more"]);
        Ok(())
    }

    #[test]
    async fn pagination_covers_every_row_exactly_once() -> Result<()> {
        let pool = pool();
        // Every place shares name, rank and distance, so only the `id` tiebreaker
        // makes the order total. Without it, consecutive pages are independent SQL
        // runs whose tied rows can reshuffle, skipping or duplicating a row.
        for id in 1..=5 {
            insert_place(id, &[("name", "Hamburg")], 53.5, 9.9, &pool).await;
        }
        let app = app!(pool);

        let mut seen = Vec::new();
        for offset in 0..5 {
            let req = TestRequest::get()
                .uri(&format!(
                    "/search?q=hamburg&type_filter=place&limit=1&offset={offset}"
                ))
                .to_request();
            let page: Value = test::call_and_read_body_json(&app, req).await;
            let rows = page["results"].as_array().unwrap();
            assert_eq!(1, rows.len(), "page at offset {offset}");
            seen.push(rows[0]["id"].as_i64().unwrap());
        }

        seen.sort_unstable();
        // Every id 1..=5, each exactly once — no skips, no duplicates.
        assert_eq!(vec![1, 2, 3, 4, 5], seen);
        Ok(())
    }

    #[test]
    async fn nearer_place_wins_a_rank_tie() -> Result<()> {
        let pool = pool();
        insert_place(
            1,
            &[("name", "Far"), ("addr:city", "Hamburg")],
            60.0,
            9.9,
            &pool,
        )
        .await;
        insert_place(
            2,
            &[("name", "Near"), ("addr:city", "Hamburg")],
            53.6,
            9.9,
            &pool,
        )
        .await;
        let app = app!(pool);
        let req = TestRequest::get()
            .uri("/search?q=hamburg&lat=53.5&lon=9.9")
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!("Near", res["results"][0]["name"]);
        assert_eq!("Far", res["results"][1]["name"]);
        Ok(())
    }
}
