use super::Report;
use crate::log;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::web::Redirect;
use actix_web::Either;
use actix_web::HttpRequest;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
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

impl Into<GetItem> for Report {
    fn into(self) -> GetItem {
        let area_id = if self.area_url_alias == "earth" {
            "".into()
        } else {
            self.area_url_alias
        };

        GetItem {
            id: self.id,
            area_id,
            date: self.date.to_string(),
            tags: self.tags,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self
                .deleted_at
                .map(|it| it.format(&Rfc3339).unwrap())
                .unwrap_or_default()
                .into(),
        }
    }
}

impl Into<Json<GetItem>> for Report {
    fn into(self) -> Json<GetItem> {
        Json(self.into())
    }
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Either<Json<Vec<GetItem>>, Redirect>, Error> {
    let started_at = Instant::now();
    if args.limit.is_none() && args.updated_since.is_none() {
        return Ok(Either::Right(
            Redirect::to("https://static.btcmap.org/api/v2/reports.json").permanent(),
        ));
    }
    let reports = pool
        .get()
        .await?
        .interact(move |conn| match &args.updated_since {
            Some(updated_since) => Report::select_updated_since(updated_since, args.limit, conn),
            None => Report::select_updated_since(
                &OffsetDateTime::now_utc()
                    .checked_sub(Duration::days(7))
                    .unwrap(),
                args.limit,
                conn,
            ),
        })
        .await??;
    let reports_len = reports.len() as i64;
    let res = Either::Left(Json(reports.into_iter().map(|it| it.into()).collect()));
    let time_ms = Instant::now().duration_since(started_at).as_millis() as i64;
    log::log_sync_api_request(&req, "v2/reports", reports_len, time_ms)?;
    Ok(res)
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Arc<Pool>>) -> Result<Json<GetItem>, Error> {
    let id = id.into_inner();
    pool.get()
        .await?
        .interact(move |conn| Report::select_by_id(id, conn))
        .await??
        .map(|it| it.into())
        .ok_or(Error::HttpNotFound(format!(
            "Report with id = {id} doesn't exist"
        )))
}

#[cfg(test)]
mod test {
    use crate::area::Area;
    use crate::report::v2::GetItem;
    use crate::report::Report;
    use crate::test::mock_state;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use geojson::{Feature, GeoJson};
    use serde_json::{Map, Value};
    use time::macros::{date, datetime};
    use time::OffsetDateTime;

    #[test]
    async fn get_empty_table() -> Result<()> {
        let state = mock_state().await;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
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
        let state = mock_state().await;
        let mut area_tags = Map::new();
        area_tags.insert("url_alias".into(), "test".into());
        Area::insert(GeoJson::Feature(Feature::default()), area_tags, &state.conn)?;
        Report::insert(
            1,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &state.conn,
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
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
        let state = mock_state().await;
        let mut area_tags = Map::new();
        area_tags.insert("url_alias".into(), "test".into());
        Area::insert(GeoJson::Feature(Feature::default()), area_tags, &state.conn)?;
        Report::insert(1, &date!(2023 - 05 - 06), &Map::new(), &state.conn)?;
        Report::insert(1, &date!(2023 - 05 - 07), &Map::new(), &state.conn)?;
        Report::insert(1, &date!(2023 - 05 - 08), &Map::new(), &state.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
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
        let state = mock_state().await;
        let mut area_tags = Map::new();
        area_tags.insert("url_alias".into(), "test".into());
        Area::insert(GeoJson::Feature(Feature::default()), area_tags, &state.conn)?;
        let report_1 = Report::insert(
            1,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &state.conn,
        )?;
        Report::_set_updated_at(
            report_1.id,
            &datetime!(2022-01-05 00:00:00 UTC),
            &state.conn,
        )?;
        let report_2 = Report::insert(
            1,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &state.conn,
        )?;
        Report::_set_updated_at(
            report_2.id,
            &datetime!(2022-02-05 00:00:00 UTC),
            &state.conn,
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
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
