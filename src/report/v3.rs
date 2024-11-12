use super::Report;
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
    let reports = pool
        .get()
        .await?
        .interact(move |conn| {
            Report::select_updated_since(&args.updated_since, Some(args.limit), conn)
        })
        .await??;
    req.extensions_mut()
        .insert(RequestExtension::new(reports.len()));
    Ok(Json(reports.into_iter().map(|it| it.into()).collect()))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Pool>) -> Result<Json<GetItem>, Error> {
    let id = id.into_inner();
    pool.get()
        .await?
        .interact(move |conn| Report::select_by_id(id, conn))
        .await??
        .map(|it| it.into())
        .ok_or(Error::NotFound(format!(
            "Report with id = {id} doesn't exist"
        )))
}

#[cfg(test)]
mod test {
    use crate::area::Area;
    use crate::error::{self, SyncAPIErrorResponseBody};
    use crate::report::Report;
    use crate::test::mock_db;
    use crate::Result;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data, QueryConfig};
    use actix_web::{test, App};
    use serde_json::Map;
    use time::macros::datetime;
    use time::OffsetDateTime;

    #[test]
    async fn get_no_updated_since() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(QueryConfig::default().error_handler(error::query_error_handler))
                .app_data(Data::new(mock_db().await.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=1").to_request();
        let res: SyncAPIErrorResponseBody =
            test::try_call_and_read_body_json(&app, req).await.unwrap();
        assert_eq!(StatusCode::BAD_REQUEST.as_u16(), res.http_code);
        assert!(res.message.contains("missing field `updated_since`"));
        Ok(())
    }

    #[test]
    async fn get_no_limit() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(QueryConfig::default().error_handler(error::query_error_handler))
                .app_data(Data::new(mock_db().await.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z")
            .to_request();
        let res: SyncAPIErrorResponseBody =
            test::try_call_and_read_body_json(&app, req).await.unwrap();
        assert_eq!(StatusCode::BAD_REQUEST.as_u16(), res.http_code);
        assert!(res.message.contains("missing field `limit`"));
        Ok(())
    }

    #[test]
    async fn get_empty_array() -> Result<()> {
        let db = mock_db().await;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
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
        let db = mock_db().await;
        let area = Area::insert(Area::mock_tags(), &db.conn)?.unwrap();
        let report = Report::insert(
            area.id,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &db.conn,
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
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
        let db = mock_db().await;
        let area = Area::insert(Area::mock_tags(), &db.conn)?.unwrap();
        let report_1 = Report::insert(
            area.id,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &db.conn,
        )?;
        let report_2 = Report::insert(
            area.id,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &db.conn,
        )?;
        let _report_3 = Report::insert(
            area.id,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &db.conn,
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
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
        let db = mock_db().await;
        let area = Area::insert(Area::mock_tags(), &db.conn)?.unwrap();
        let report_1 = Report::insert(
            area.id,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &db.conn,
        )?;
        Report::_set_updated_at(report_1.id, &datetime!(2022-01-05 00:00 UTC), &db.conn)?;
        let report_2 = Report::insert(
            area.id,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &db.conn,
        )?;
        let report_2 =
            Report::_set_updated_at(report_2.id, &datetime!(2022-02-05 00:00 UTC), &db.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
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
