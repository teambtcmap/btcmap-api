use crate::db;
use crate::db::area::schema::Area;
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
    pub tags: Option<Map<String, Value>>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<Area> for GetItem {
    fn from(val: Area) -> Self {
        let tags = if val.deleted_at.is_none() && !val.tags.is_empty() {
            Some(val.tags)
        } else {
            None
        };
        GetItem {
            id: val.id,
            tags,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

impl From<Area> for Json<GetItem> {
    fn from(val: Area) -> Self {
        Json(val.into())
    }
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetArgs>,
    pool: Data<Pool>,
) -> Result<Json<Vec<GetItem>>, Error> {
    let areas =
        db::area::queries_async::select(Some(args.updated_since), true, Some(args.limit), &pool)
            .await?;
    req.extensions_mut()
        .insert(RequestExtension::new(areas.len()));
    Ok(Json(areas.into_iter().map(|it| it.into()).collect()))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<String>, pool: Data<Pool>) -> Result<Json<GetItem>, Error> {
    db::area::queries_async::select_by_id_or_alias(id.to_string(), &pool)
        .await
        .map(Into::into)
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
    use time::macros::datetime;

    #[test]
    async fn get_no_updated_since() -> Result<()> {
        let app = test::init_service(App::new().service(scope("/").service(super::get))).await;
        let req = TestRequest::get().uri("/?limit=1").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(StatusCode::BAD_REQUEST, res.status());
        Ok(())
    }

    #[test]
    async fn get_no_limit() -> Result<()> {
        let app = test::init_service(App::new().service(scope("/").service(super::get))).await;
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
        let area = db::area::queries_async::insert(Area::mock_tags(), &pool).await?;
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
        assert_eq!(res, vec![area.into()]);
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let pool = pool();
        let area_1 = db::area::queries_async::insert(Area::mock_tags(), &pool).await?;
        let area_2 = db::area::queries_async::insert(Area::mock_tags(), &pool).await?;
        let _area_3 = db::area::queries_async::insert(Area::mock_tags(), &pool).await?;
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
        assert_eq!(res, vec![area_1.into(), area_2.into()]);
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let pool = pool();
        let area_1 = db::area::queries_async::insert(Area::mock_tags(), &pool).await?;
        db::area::queries_async::set_updated_at(area_1.id, datetime!(2022-01-05 00:00 UTC), &pool)
            .await?;
        let area_2 = db::area::queries_async::insert(Area::mock_tags(), &pool).await?;
        let area_2 = db::area::queries_async::set_updated_at(
            area_2.id,
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
        assert_eq!(res, vec![area_2.into()]);
        Ok(())
    }
}
