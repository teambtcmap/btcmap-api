use crate::db;
use crate::db::report::schema::Report;
use crate::log::RequestExtension;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::HttpMessage;
use actix_web::HttpRequest;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetArgs {
    #[serde(with = "time::serde::rfc3339")]
    updated_since: OffsetDateTime,
    limit: i64,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct GetItem {
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Map<String, Value>>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<Report> for GetItem {
    fn from(val: Report) -> Self {
        let area_id = if val.deleted_at.is_none() {
            Some(val.area_id)
        } else {
            None
        };
        let date = if val.deleted_at.is_none() {
            Some(val.date)
        } else {
            None
        };
        let tags = if val.deleted_at.is_none() && !val.tags.is_empty() {
            Some(val.tags)
        } else {
            None
        };
        GetItem {
            id: val.id,
            area_id,
            date: date.map(|it| it.to_string()),
            tags,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

impl From<Report> for Json<GetItem> {
    fn from(val: Report) -> Self {
        Json(val.into())
    }
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetArgs>,
    pool: Data<Pool>,
) -> Result<Json<Vec<GetItem>>, Error> {
    let reports =
        db::report::queries::select_updated_since(args.updated_since, Some(args.limit), &pool)
            .await?;
    req.extensions_mut()
        .insert(RequestExtension::new(reports.len()));
    Ok(Json(reports.into_iter().map(|it| it.into()).collect()))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Pool>) -> Result<Json<GetItem>, Error> {
    db::report::queries::select_by_id(*id, &pool)
        .await
        .map(|it| it.into())
}

#[cfg(test)]
mod test {
    use crate::db::area::schema::Area;
    use crate::db::test::pool;
    use crate::{db, Result};
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use serde_json::Map;
    use time::macros::datetime;
    use time::OffsetDateTime;

    #[test]
    async fn get_no_updated_since() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=1").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(StatusCode::BAD_REQUEST, res.status());
        Ok(())
    }

    #[test]
    async fn get_no_limit() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z")
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(StatusCode::BAD_REQUEST, res.status());
        Ok(())
    }

    #[test]
    async fn get_empty_array() -> Result<()> {
        let pool = pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=1")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 0);
        Ok(())
    }

    #[test]
    async fn get_not_empty_array() -> Result<()> {
        let pool = pool();
        let area = db::area::queries::insert(Area::mock_tags(), &pool).await?;
        let report = db::report::queries::insert(
            area.id,
            OffsetDateTime::now_utc().date(),
            Map::new(),
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
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=1")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res, vec![report.into()]);
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let pool = pool();
        let area = db::area::queries::insert(Area::mock_tags(), &pool).await?;
        let report_1 = db::report::queries::insert(
            area.id,
            OffsetDateTime::now_utc().date(),
            Map::new(),
            &pool,
        )
        .await?;
        let report_2 = db::report::queries::insert(
            area.id,
            OffsetDateTime::now_utc().date(),
            Map::new(),
            &pool,
        )
        .await?;
        let _report_3 = db::report::queries::insert(
            area.id,
            OffsetDateTime::now_utc().date(),
            Map::new(),
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
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=2")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res, vec![report_1.into(), report_2.into()]);
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let pool = pool();
        let area = db::area::queries::insert(Area::mock_tags(), &pool).await?;
        let report_1 = db::report::queries::insert(
            area.id,
            OffsetDateTime::now_utc().date(),
            Map::new(),
            &pool,
        )
        .await?;
        db::report::queries::set_updated_at(report_1.id, datetime!(2022-01-05 00:00 UTC), &pool)
            .await?;
        let report_2 = db::report::queries::insert(
            area.id,
            OffsetDateTime::now_utc().date(),
            Map::new(),
            &pool,
        )
        .await?;
        let report_2 = db::report::queries::set_updated_at(
            report_2.id,
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
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res, vec![report_2.into()]);
        Ok(())
    }
}
