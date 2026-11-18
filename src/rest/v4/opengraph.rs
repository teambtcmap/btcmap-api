use crate::db;
use crate::service;
use crate::{db::image::ImagePool, rest::error::RestApiError};
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse,
};
use time::{Duration, OffsetDateTime};
use tracing::warn;

/// How long a rendered card is served before the element is fetched and the
/// image rendered again. OSM edits are rare and these cards are decorative, so
/// a week is plenty.
const CACHE_TTL: Duration = Duration::days(7);

#[get("/osm-element/{id}")]
pub async fn osm_element_get(
    id: Path<String>,
    image_pool: Data<ImagePool>,
) -> Result<HttpResponse, RestApiError> {
    let Some((osm_type, osm_id)) = parse_osm_id(&id) else {
        return Ok(HttpResponse::BadRequest().body("Expected node:ID, way:ID or relation:ID"));
    };

    let now = OffsetDateTime::now_utc();
    let cache_cutoff = now - CACHE_TTL;
    match db::image::osm_element_og::queries::select_by_osm_id(osm_type, osm_id, &image_pool).await
    {
        Ok(Some(cached)) if is_fresh(cached.created_at, now) => {
            return Ok(HttpResponse::Ok()
                .content_type("image/jpeg")
                .body(cached.image_data));
        }
        Ok(_) => {}
        // A broken cache shouldn't take the endpoint down, render a fresh image.
        Err(err) => warn!(error = %err, "failed to read OSM OG cache"),
    }

    let payload = match service::og::osm_og(osm_type, osm_id).await {
        Ok(Some(payload)) => payload,
        Ok(None) => return Ok(HttpResponse::NotFound().body("OSM element not found")),
        Err(err) => {
            warn!(error = %err, "failed to render OSM OG image");
            return Ok(HttpResponse::BadGateway().body("Failed to fetch OSM element"));
        }
    };

    if let Err(err) = db::image::osm_element_og::queries::upsert(
        osm_type,
        osm_id,
        payload.data.clone(),
        payload.width,
        payload.height,
        payload.data.len() as i64,
        &image_pool,
    )
    .await
    {
        warn!(error = %err, "failed to write OSM OG cache");
    }
    // Unique ids are unbounded, so drop entries that expired instead of letting
    // the table grow forever.
    if let Err(err) =
        db::image::osm_element_og::queries::delete_created_before(cache_cutoff, &image_pool).await
    {
        warn!(error = %err, "failed to prune OSM OG cache");
    }

    Ok(HttpResponse::Ok()
        .content_type("image/jpeg")
        .body(payload.data))
}

/// Parse an `<type>:<id>` OSM ID, e.g. `node:1` or `way:346185092`.
fn parse_osm_id(id: &str) -> Option<(&str, i64)> {
    let (osm_type, osm_id) = id.split_once(':')?;
    if !matches!(osm_type, "node" | "way" | "relation") {
        return None;
    }
    let osm_id: i64 = osm_id.parse().ok()?;
    (osm_id > 0).then_some((osm_type, osm_id))
}

fn is_fresh(created_at: OffsetDateTime, now: OffsetDateTime) -> bool {
    created_at > now - CACHE_TTL
}

#[cfg(test)]
mod test {
    use super::is_fresh;
    use super::parse_osm_id;
    use time::{Duration, OffsetDateTime};

    #[test]
    fn parse_osm_ref_accepts_known_types() {
        assert_eq!(Some(("node", 1)), parse_osm_id("node:1"));
        assert_eq!(Some(("way", 346185092)), parse_osm_id("way:346185092"));
        assert_eq!(Some(("relation", 62422)), parse_osm_id("relation:62422"));
    }

    #[test]
    fn parse_osm_ref_rejects_invalid_input() {
        assert_eq!(None, parse_osm_id("123"));
        assert_eq!(None, parse_osm_id("node:"));
        assert_eq!(None, parse_osm_id("node:abc"));
        assert_eq!(None, parse_osm_id("node:0"));
        assert_eq!(None, parse_osm_id("node:-1"));
        assert_eq!(None, parse_osm_id("changeset:1"));
        assert_eq!(None, parse_osm_id("node:1:2"));
    }

    #[test]
    fn is_fresh_respects_ttl() {
        let now = OffsetDateTime::now_utc();
        assert!(is_fresh(now, now));
        assert!(is_fresh(now - Duration::days(6), now));
        assert!(!is_fresh(now - Duration::days(7), now));
        assert!(!is_fresh(now - Duration::days(8), now));
    }
}
