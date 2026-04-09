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

    let events = db::main::element_event::queries::select_created_between(
        day_ago,
        now + Duration::seconds(1),
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;

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

    let comments = db::main::element_comment::queries::select_created_between(
        day_ago,
        now + Duration::seconds(1),
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;

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
}
