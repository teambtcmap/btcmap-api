use crate::db;
use crate::db::main::place_submission::schema::PlaceSubmission;
use crate::db::main::MainPool;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Query;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{Map, Value};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Args {
    pub source: Option<String>,
}

#[derive(Serialize, ts_rs::TS)]
#[ts(export, rename = "PlaceSubmission")]
pub struct Item {
    #[ts(type = "number")]
    pub id: i64,
    pub origin: String,
    pub external_id: String,
    pub lat: f64,
    pub lon: f64,
    pub category: String,
    pub name: String,
    #[ts(type = "Record<string, unknown>")]
    pub extra_fields: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket_url: Option<String>,
    pub revoked: bool,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub updated_at: OffsetDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    #[ts(optional, type = "string")]
    pub closed_at: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    #[ts(optional, type = "string")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<PlaceSubmission> for Item {
    fn from(val: PlaceSubmission) -> Self {
        Item {
            id: val.id,
            origin: val.origin,
            external_id: val.external_id,
            lat: val.lat,
            lon: val.lon,
            category: val.category,
            name: val.name,
            extra_fields: val.extra_fields,
            ticket_url: val.ticket_url.map(humanize_ticket_url),
            revoked: val.revoked,
            created_at: val.created_at,
            updated_at: val.updated_at,
            closed_at: val.closed_at,
            deleted_at: val.deleted_at,
        }
    }
}

fn humanize_ticket_url(url: String) -> String {
    url.replacen("/api/v1/repos", "", 1)
}

impl From<PlaceSubmission> for Json<Item> {
    fn from(val: PlaceSubmission) -> Self {
        Json(val.into())
    }
}

#[get("")]
pub async fn get(args: Query<Args>, pool: Data<MainPool>) -> RestResult<Vec<Item>> {
    let items = match args.source.as_deref() {
        Some(origin) => {
            db::main::place_submission::queries::select_open_and_not_revoked_by_origin(
                origin.to_string(),
                &pool,
            )
            .await
        }
        None => db::main::place_submission::queries::select_open_and_not_revoked(&pool).await,
    }
    .map_err(|_| RestApiError::database())?;
    Ok(Json(items.into_iter().map(Into::into).collect()))
}

#[cfg(test)]
mod test {
    use crate::db::main::place_submission::blocking_queries::InsertArgs;
    use crate::db::main::test::pool;
    use crate::{db, Result};
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use serde_json::Map;

    #[test]
    async fn get_empty_array() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<serde_json::Value> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_returns_open_and_not_revoked() -> Result<()> {
        let pool = pool();

        let open_args = InsertArgs {
            origin: "coinos".to_string(),
            external_id: "1".to_string(),
            lat: 1.23,
            lon: 4.56,
            category: "cafe".to_string(),
            name: "Open Place".to_string(),
            extra_fields: Map::new(),
        };
        let open = db::main::place_submission::queries::insert(open_args, &pool).await?;

        let revoked_args = InsertArgs {
            origin: "coinos".to_string(),
            external_id: "2".to_string(),
            lat: 1.23,
            lon: 4.56,
            category: "cafe".to_string(),
            name: "Revoked Place".to_string(),
            extra_fields: Map::new(),
        };
        let revoked = db::main::place_submission::queries::insert(revoked_args, &pool).await?;
        db::main::place_submission::queries::set_revoked(revoked.id, true, &pool).await?;

        let closed_args = InsertArgs {
            origin: "coinos".to_string(),
            external_id: "3".to_string(),
            lat: 1.23,
            lon: 4.56,
            category: "cafe".to_string(),
            name: "Closed Place".to_string(),
            extra_fields: Map::new(),
        };
        let closed = db::main::place_submission::queries::insert(closed_args, &pool).await?;
        db::main::place_submission::queries::set_closed_at(
            closed.id,
            Some(time::OffsetDateTime::now_utc()),
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
        let res: Vec<serde_json::Value> = test::call_and_read_body_json(&app, req).await;

        assert_eq!(1, res.len());
        assert_eq!(open.id, res[0]["id"].as_i64().unwrap());
        assert_eq!(false, res[0]["revoked"].as_bool().unwrap());
        assert!(res[0]["closed_at"].is_null());

        Ok(())
    }

    #[test]
    async fn get_filters_by_source() -> Result<()> {
        let pool = pool();

        for (origin, external_id) in [("square", "1"), ("coinos", "1"), ("coinos", "2")] {
            let args = InsertArgs {
                origin: origin.to_string(),
                external_id: external_id.to_string(),
                lat: 1.23,
                lon: 4.56,
                category: "cafe".to_string(),
                name: format!("{origin} place"),
                extra_fields: Map::new(),
            };
            db::main::place_submission::queries::insert(args, &pool).await?;
        }

        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;

        let req = TestRequest::get().uri("/?source=coinos").to_request();
        let res: Vec<serde_json::Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, res.len());
        assert!(res.iter().all(|r| r["origin"] == "coinos"));

        let req = TestRequest::get().uri("/?source=square").to_request();
        let res: Vec<serde_json::Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!("square", res[0]["origin"]);

        let req = TestRequest::get()
            .uri("/?source=does_not_exist")
            .to_request();
        let res: Vec<serde_json::Value> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());

        Ok(())
    }

    #[test]
    async fn get_rewrites_ticket_url_to_web_link() -> Result<()> {
        let pool = pool();
        let args = InsertArgs {
            origin: "square".to_string(),
            external_id: "url-rewrite".to_string(),
            lat: 1.23,
            lon: 4.56,
            category: "cafe".to_string(),
            name: "URL rewrite probe".to_string(),
            extra_fields: Map::new(),
        };
        let submission = db::main::place_submission::queries::insert(args, &pool).await?;
        let api_url = "https://gitea.btcmap.org/api/v1/repos/teambtcmap/btcmap-data/issues/42";
        db::main::place_submission::queries::set_ticket_url(
            submission.id,
            api_url.to_string(),
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
        let res: Vec<serde_json::Value> = test::call_and_read_body_json(&app, req).await;

        assert_eq!(
            "https://gitea.btcmap.org/teambtcmap/btcmap-data/issues/42",
            res[0]["ticket_url"].as_str().unwrap(),
        );
        Ok(())
    }

    #[test]
    async fn humanize_ticket_url_strips_api_prefix() {
        assert_eq!(
            "https://gitea.btcmap.org/teambtcmap/btcmap-data/issues/1",
            super::humanize_ticket_url(
                "https://gitea.btcmap.org/api/v1/repos/teambtcmap/btcmap-data/issues/1".to_string(),
            ),
        );
    }

    #[test]
    async fn humanize_ticket_url_leaves_non_gitea_urls_alone() {
        let url = "https://example.com/issue/123".to_string();
        assert_eq!(url.clone(), super::humanize_ticket_url(url));
    }

    #[test]
    async fn humanize_ticket_url_strips_only_first_occurrence() {
        assert_eq!(
            "https://gitea.btcmap.org/foo/api/v1/repos/bar",
            super::humanize_ticket_url(
                "https://gitea.btcmap.org/api/v1/repos/foo/api/v1/repos/bar".to_string(),
            ),
        );
    }
}
