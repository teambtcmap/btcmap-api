use crate::db;
use crate::db::main::invoice::schema::{InvoiceStatus, InvoicedService};
use crate::db::main::MainPool;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Query;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::LazyLock;
use time::Duration;
use time::OffsetDateTime;

static TIP_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(lightning:[^)]+)").unwrap());

const EVENT_TYPE_CREATE: &str = "place_added";
const EVENT_TYPE_UPDATE: &str = "place_updated";
const EVENT_TYPE_DELETE: &str = "place_deleted";
const EVENT_TYPE_COMMENT: &str = "place_commented";
const EVENT_TYPE_BOOST: &str = "place_boosted";

#[derive(Deserialize)]
pub struct GetActivityArgs {
    days: Option<i64>,
    area: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ActivityItem {
    pub r#type: String,
    pub place_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub osm_user_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub osm_user_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub osm_user_tip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_days: Option<i64>,
    pub image: String,
    #[serde(with = "time::serde::rfc3339", rename = "date")]
    pub created_at: OffsetDateTime,
}

fn get_event_type(r#type: &str) -> String {
    match r#type {
        "create" => EVENT_TYPE_CREATE.to_string(),
        "update" => EVENT_TYPE_UPDATE.to_string(),
        "delete" => EVENT_TYPE_DELETE.to_string(),
        _ => String::new(),
    }
}

#[get("")]
pub async fn get(
    args: Query<GetActivityArgs>,
    pool: Data<MainPool>,
) -> RestResult<Vec<ActivityItem>> {
    let now = OffsetDateTime::now_utc();
    let days = args.days.unwrap_or(1);
    let day_ago = now.saturating_sub(Duration::days(days));
    let period_end = now + Duration::seconds(1);

    // Resolve area if specified, build element ID set for boost filtering
    let area_id: Option<i64> = if let Some(ref area_id_or_alias) = args.area {
        let area = db::main::area::queries::select_by_id_or_alias(area_id_or_alias.clone(), &pool)
            .await
            .map_err(|e| match e {
                crate::Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => {
                    RestApiError::not_found()
                }
                _ => RestApiError::database(),
            })?;
        Some(area.id)
    } else {
        None
    };

    // For boost filtering we need element IDs (invoices store element_id in a
    // description string, not a JOINable column)
    let area_element_ids: Option<HashSet<i64>> = if let Some(area_id) = area_id {
        let area_elements = db::main::area_element::queries::select_by_area_id(area_id, &pool)
            .await
            .map_err(|_| RestApiError::database())?;
        Some(
            area_elements
                .into_iter()
                .filter(|ae| ae.deleted_at.is_none())
                .map(|ae| ae.element_id)
                .collect(),
        )
    } else {
        None
    };

    let in_area = |element_id: i64| -> bool {
        match &area_element_ids {
            Some(ids) => ids.contains(&element_id),
            None => true,
        }
    };

    // Fetch events — area-scoped or global
    let events = if let Some(area_id) = area_id {
        db::main::element_event::queries::select_created_between_for_area(
            area_id, day_ago, period_end, &pool,
        )
        .await
        .map_err(|_| RestApiError::database())?
    } else {
        db::main::element_event::queries::select_created_between(day_ago, period_end, &pool)
            .await
            .map_err(|_| RestApiError::database())?
    };

    let mut items: Vec<ActivityItem> = Vec::with_capacity(events.len());
    for event in events {
        let element = db::main::element::queries::select_by_id(event.element_id, &pool)
            .await
            .map_err(|_| RestApiError::database())?;

        let osm_user = db::main::osm_user::queries::select_by_id(event.user_id, &pool)
            .await
            .map_err(|_| RestApiError::database())?;

        let element_name = element.name(None);

        let user_tip = TIP_RE
            .captures(&osm_user.osm_data.description)
            .map(|c| c[1].to_string());

        items.push(ActivityItem {
            r#type: get_event_type(&event.r#type),
            place_id: event.element_id,
            place_name: Some(element_name),
            osm_user_id: Some(event.user_id),
            osm_user_name: Some(osm_user.osm_data.display_name),
            osm_user_tip: user_tip,
            comment: None,
            duration_days: None,
            image: format!("https://api.btcmap.org/og/element/{}", event.element_id),
            created_at: event.created_at,
        });
    }

    // Fetch comments — area-scoped or global
    let comments = if let Some(area_id) = area_id {
        db::main::element_comment::queries::select_created_between_for_area(
            area_id, day_ago, period_end, &pool,
        )
        .await
        .map_err(|_| RestApiError::database())?
    } else {
        db::main::element_comment::queries::select_created_between(day_ago, period_end, &pool)
            .await
            .map_err(|_| RestApiError::database())?
    };

    for comment in comments {
        if comment.deleted_at.is_some() {
            continue;
        }

        let element = db::main::element::queries::select_by_id(comment.element_id, &pool)
            .await
            .map_err(|_| RestApiError::database())?;

        let element_name = element.name(None);

        items.push(ActivityItem {
            r#type: EVENT_TYPE_COMMENT.to_string(),
            place_id: comment.element_id,
            place_name: Some(element_name),
            osm_user_id: None,
            osm_user_name: None,
            osm_user_tip: None,
            comment: Some(comment.comment),
            duration_days: None,
            image: format!("https://api.btcmap.org/og/element/{}", comment.element_id),
            created_at: comment.created_at,
        });
    }

    // Fetch boosts — invoices store element_id in a description string,
    // not a JOINable column, so we use the in_area() helper for filtering
    let paid_invoices = db::main::invoice::queries::select_by_status(InvoiceStatus::Paid, &pool)
        .await
        .map_err(|_| RestApiError::database())?;

    for invoice in paid_invoices {
        let service = InvoicedService::from_description(&invoice.description);
        let InvoicedService::Boost {
            element_id,
            duration_days,
        } = service
        else {
            continue;
        };

        if !in_area(element_id) {
            continue;
        }

        let created_at = OffsetDateTime::parse(
            &invoice.created_at,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|_| RestApiError::database())?;

        if created_at < day_ago || created_at > now {
            continue;
        }

        let element = match db::main::element::queries::select_by_id(element_id, &pool).await {
            Ok(e) => e,
            Err(_) => continue,
        };

        let element_name = element.name(None);

        items.push(ActivityItem {
            r#type: EVENT_TYPE_BOOST.to_string(),
            place_id: element_id,
            place_name: Some(element_name),
            osm_user_id: None,
            osm_user_name: None,
            osm_user_tip: None,
            comment: None,
            duration_days: Some(duration_days),
            image: format!("https://api.btcmap.org/og/element/{element_id}"),
            created_at,
        });
    }

    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(Json(items))
}

#[cfg(test)]
mod test {
    use crate::db::main::test::pool;
    use crate::service::overpass::OverpassElement;
    use crate::{db, Result};
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};

    #[test]
    async fn get_empty_array() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<super::ActivityItem> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_with_events() -> Result<()> {
        let pool = pool();
        let user = db::main::osm_user::queries::insert(
            1,
            crate::service::osm::EditingApiUser::mock(),
            &pool,
        )
        .await?;
        let element = db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let _event =
            db::main::element_event::queries::insert(user.id, element.id, "create", &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<super::ActivityItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(super::EVENT_TYPE_CREATE, res[0].r#type);
        Ok(())
    }

    #[test]
    async fn get_returns_events_from_last_day() -> Result<()> {
        let pool = pool();
        let user = db::main::osm_user::queries::insert(
            1,
            crate::service::osm::EditingApiUser::mock(),
            &pool,
        )
        .await?;
        let element = db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let event =
            db::main::element_event::queries::insert(user.id, element.id, "create", &pool).await?;

        pool.get()
            .await?
            .interact(move |conn| {
                conn.execute(
                    "UPDATE element_event SET created_at = '2020-01-01T00:00:00Z' WHERE id = ?1",
                    rusqlite::params![event.id],
                )
            })
            .await??;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<super::ActivityItem> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_with_comments() -> Result<()> {
        let pool = pool();
        let element = db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let _comment =
            db::main::element_comment::queries::insert(element.id, "Test comment", &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<super::ActivityItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(super::EVENT_TYPE_COMMENT, res[0].r#type);
        assert_eq!(Some("Test comment".to_string()), res[0].comment);
        Ok(())
    }

    #[test]
    async fn get_mixed_events_and_comments_sorted_by_date() -> Result<()> {
        let pool = pool();
        let user = db::main::osm_user::queries::insert(
            1,
            crate::service::osm::EditingApiUser::mock(),
            &pool,
        )
        .await?;
        let element = db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;

        let comment =
            db::main::element_comment::queries::insert(element.id, "Older comment", &pool).await?;

        pool.get()
            .await?
            .interact(move |conn| {
                conn.execute(
                    "UPDATE element_comment SET created_at = datetime('now', '-1 hour') WHERE id = ?1",
                    rusqlite::params![comment.id],
                )
            })
            .await??;

        let _event =
            db::main::element_event::queries::insert(user.id, element.id, "create", &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<super::ActivityItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, res.len());
        assert_eq!(super::EVENT_TYPE_CREATE, res[0].r#type);
        assert_eq!(super::EVENT_TYPE_COMMENT, res[1].r#type);
        Ok(())
    }

    #[test]
    async fn get_with_boosts() -> Result<()> {
        let pool = pool();
        let element = db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let _invoice = db::main::invoice::queries::insert(
            "src",
            format!("element_boost:{}:30", element.id),
            1000,
            "hash",
            "req",
            db::main::invoice::schema::InvoiceStatus::Paid,
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
        let res: Vec<super::ActivityItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(super::EVENT_TYPE_BOOST, res[0].r#type);
        assert_eq!(Some(30), res[0].duration_days);
        Ok(())
    }

    #[test]
    async fn get_filtered_by_area() -> Result<()> {
        let pool = pool();
        let user = db::main::osm_user::queries::insert(
            1,
            crate::service::osm::EditingApiUser::mock(),
            &pool,
        )
        .await?;

        let element_in_area =
            db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let element_outside =
            db::main::element::queries::insert(OverpassElement::mock(2), &pool).await?;

        let area =
            db::main::area::queries::insert(db::main::area::schema::Area::mock_tags(), &pool)
                .await?;
        db::main::area_element::queries::insert(area.id, element_in_area.id, &pool).await?;

        db::main::element_event::queries::insert(user.id, element_in_area.id, "create", &pool)
            .await?;
        db::main::element_event::queries::insert(user.id, element_outside.id, "create", &pool)
            .await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;

        // Without area filter: both events
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<super::ActivityItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, res.len());

        // With area filter: only the event for the element in the area
        let req = TestRequest::get()
            .uri(&format!("/?area={}", area.id))
            .to_request();
        let res: Vec<super::ActivityItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(element_in_area.id, res[0].place_id);

        // With area alias
        let req = TestRequest::get().uri("/?area=alias").to_request();
        let res: Vec<super::ActivityItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());

        Ok(())
    }

    #[test]
    async fn get_comments_filtered_by_area() -> Result<()> {
        let pool = pool();

        let element_in_area =
            db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let element_outside =
            db::main::element::queries::insert(OverpassElement::mock(2), &pool).await?;

        let area =
            db::main::area::queries::insert(db::main::area::schema::Area::mock_tags(), &pool)
                .await?;
        db::main::area_element::queries::insert(area.id, element_in_area.id, &pool).await?;

        db::main::element_comment::queries::insert(element_in_area.id, "In area", &pool).await?;
        db::main::element_comment::queries::insert(element_outside.id, "Outside", &pool).await?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;

        let req = TestRequest::get()
            .uri(&format!("/?area={}", area.id))
            .to_request();
        let res: Vec<super::ActivityItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(super::EVENT_TYPE_COMMENT, res[0].r#type);
        assert_eq!(Some("In area".to_string()), res[0].comment);
        Ok(())
    }

    #[test]
    async fn get_boosts_filtered_by_area() -> Result<()> {
        let pool = pool();

        let element_in_area =
            db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let element_outside =
            db::main::element::queries::insert(OverpassElement::mock(2), &pool).await?;

        let area =
            db::main::area::queries::insert(db::main::area::schema::Area::mock_tags(), &pool)
                .await?;
        db::main::area_element::queries::insert(area.id, element_in_area.id, &pool).await?;

        db::main::invoice::queries::insert(
            "src",
            format!("element_boost:{}:30", element_in_area.id),
            1000,
            "hash1",
            "req1",
            db::main::invoice::schema::InvoiceStatus::Paid,
            &pool,
        )
        .await?;
        db::main::invoice::queries::insert(
            "src",
            format!("element_boost:{}:30", element_outside.id),
            1000,
            "hash2",
            "req2",
            db::main::invoice::schema::InvoiceStatus::Paid,
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
            .uri(&format!("/?area={}", area.id))
            .to_request();
        let res: Vec<super::ActivityItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(super::EVENT_TYPE_BOOST, res[0].r#type);
        assert_eq!(element_in_area.id, res[0].place_id);
        Ok(())
    }

    #[test]
    async fn get_area_not_found() -> Result<()> {
        let pool = pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?area=nonexistent").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(actix_web::http::StatusCode::NOT_FOUND, res.status());
        Ok(())
    }
}
