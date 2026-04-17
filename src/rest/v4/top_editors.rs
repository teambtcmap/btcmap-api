use crate::db::main::MainPool;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult as Res;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Query;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::sync::OnceLock;
use time::format_description::well_known::Rfc3339;
use time::macros::datetime;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetTopEditorsArgs {
    period_start: String,
    period_end: String,
    limit: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct TopEditor {
    pub id: i64,
    pub name: String,
    pub avatar_url: Option<String>,
    pub total_edits: i64,
    pub places_created: i64,
    pub places_updated: i64,
    pub places_deleted: i64,
    pub tip_url: Option<String>,
}

/// Spam/bot user ids excluded from "top editors" rankings. Shared with the
/// area-scoped top-editors endpoint in `super::areas` so the two views can't
/// drift apart.
pub(crate) const EXCLUDED_USER_IDS: &[i64] = &[
    9451067, 18545877, 19880430, 242345, 232801, 1778799, 21749653,
];

#[get("")]
pub async fn get(args: Query<GetTopEditorsArgs>, pool: Data<MainPool>) -> Res<Vec<TopEditor>> {
    let period_start = parse_date(&args.period_start, true)?;
    let period_end = parse_date(&args.period_end, false)?;
    let limit = validate_limit(args.limit)?;

    let editors = crate::db::main::osm_user::queries::select_most_active(
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

pub(crate) fn parse_date(date_str: &str, is_start: bool) -> Result<OffsetDateTime, RestApiError> {
    let date_str = if is_start {
        format!("{}T00:00:00Z", date_str)
    } else {
        let next_day = OffsetDateTime::parse(&format!("{}T00:00:00Z", date_str), &Rfc3339)
            .map_err(|_| RestApiError::invalid_input("Invalid date format"))?
            .saturating_add(time::Duration::days(1));
        format!("{}T00:00:00Z", next_day.date())
    };

    OffsetDateTime::parse(&date_str, &Rfc3339)
        .map_err(|_| RestApiError::invalid_input("Invalid date format"))
}

pub(crate) fn validate_limit(limit: Option<i64>) -> Result<i64, RestApiError> {
    let limit = limit.unwrap_or(100);
    if !(1..=1000).contains(&limit) {
        return Err(RestApiError::invalid_input(
            "limit must be between 1 and 1000",
        ));
    }
    Ok(limit)
}

pub(crate) fn far_future() -> OffsetDateTime {
    datetime!(2200-01-01 0:00 UTC)
}

static TIP_URL_RE: OnceLock<Regex> = OnceLock::new();

pub(crate) fn extract_tip_url(description: &str) -> Option<String> {
    let re = TIP_URL_RE.get_or_init(|| Regex::new(r"(lightning:[^)]+)").unwrap());
    re.captures(description).map(|c| c[1].to_string())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::main::test::pool;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};

    #[test]
    async fn get_empty_array() -> Result<(), Box<dyn std::error::Error>> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?period_start=2020-01-01&period_end=2020-12-31")
            .to_request();
        let res: Vec<TopEditor> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }
}
