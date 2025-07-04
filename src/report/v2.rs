use crate::db;
use crate::db::report::schema::Report;
use crate::log::RequestExtension;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::web::Redirect;
use actix_web::Either;
use actix_web::HttpMessage;
use actix_web::HttpRequest;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::Duration;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetArgs {
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    updated_since: Option<OffsetDateTime>,
    limit: Option<i64>,
    #[allow(dead_code)]
    compress: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct GetItem {
    pub id: i64,
    pub area_id: String,
    pub date: String,
    pub tags: Map<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub deleted_at: String,
}

impl From<Report> for GetItem {
    fn from(val: Report) -> Self {
        let unknown_area_id = Value::String("unknown".into());
        let area_id = val
            .tags
            .get("area_url_alias")
            .unwrap_or(&unknown_area_id)
            .as_str()
            .unwrap_or("unknown");

        let area_id = if area_id == "earth" { "" } else { area_id };

        GetItem {
            id: val.id,
            area_id: area_id.into(),
            date: val.date.to_string(),
            tags: val.tags,
            created_at: val.created_at,
            updated_at: val.updated_at,
            deleted_at: val
                .deleted_at
                .map(|it| it.format(&Rfc3339).unwrap())
                .unwrap_or_default(),
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
) -> Result<Either<Json<Vec<GetItem>>, Redirect>, Error> {
    if args.limit.is_none() && args.updated_since.is_none() {
        return Ok(Either::Right(
            Redirect::to("https://static.btcmap.org/api/v2/reports.json").permanent(),
        ));
    }
    let reports = pool
        .get()
        .await?
        .interact(move |conn| match args.updated_since {
            Some(updated_since) => {
                db::report::queries::select_updated_since(updated_since, args.limit, conn)
            }
            None => db::report::queries::select_updated_since(
                OffsetDateTime::now_utc()
                    .checked_sub(Duration::days(7))
                    .unwrap(),
                args.limit,
                conn,
            ),
        })
        .await??;
    let reports_len = reports.len();
    let res = Either::Left(Json(reports.into_iter().map(|it| it.into()).collect()));
    req.extensions_mut()
        .insert(RequestExtension::new(reports_len));
    Ok(res)
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Pool>) -> Result<Json<GetItem>, Error> {
    let id = id.into_inner();
    pool.get()
        .await?
        .interact(move |conn| db::report::queries::select_by_id(id, conn))
        .await?
        .map(|it| it.into())
}

#[cfg(test)]
mod test {
    use crate::db::area::schema::Area;
    use crate::report::v2::GetItem;
    use crate::test::{mock_db, mock_pool};
    use crate::{db, Result};
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use serde_json::{Map, Value};
    use time::macros::{date, datetime};
    use time::OffsetDateTime;

    #[test]
    async fn get_empty_table() -> Result<()> {
        let db = mock_db();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=1").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 0);
        Ok(())
    }

    #[test]
    async fn get_one_row() -> Result<()> {
        let pool = mock_pool().await;
        db::area::queries_async::insert(Area::mock_tags(), &pool).await?;
        db::report::queries_async::insert(1, OffsetDateTime::now_utc().date(), Map::new(), &pool)
            .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=100").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 1);
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let pool = mock_pool().await;
        db::area::queries_async::insert(Area::mock_tags(), &pool).await?;
        db::report::queries_async::insert(1, date!(2023 - 05 - 06), Map::new(), &pool).await?;
        db::report::queries_async::insert(1, date!(2023 - 05 - 07), Map::new(), &pool).await?;
        db::report::queries_async::insert(1, date!(2023 - 05 - 08), Map::new(), &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=2").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 2);
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let pool = mock_pool().await;
        db::area::queries_async::insert(Area::mock_tags(), &pool).await?;
        let report_1 = db::report::queries_async::insert(
            1,
            OffsetDateTime::now_utc().date(),
            Map::new(),
            &pool,
        )
        .await?;
        db::report::queries_async::set_updated_at(
            report_1.id,
            datetime!(2022-01-05 00:00:00 UTC),
            &pool,
        )
        .await?;
        let report_2 = db::report::queries_async::insert(
            1,
            OffsetDateTime::now_utc().date(),
            Map::new(),
            &pool,
        )
        .await?;
        db::report::queries_async::set_updated_at(
            report_2.id,
            datetime!(2022-02-05 00:00:00 UTC),
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
            .uri("/?updated_since=2022-01-10T00:00:00Z")
            .to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 1);
        Ok(())
    }
}
