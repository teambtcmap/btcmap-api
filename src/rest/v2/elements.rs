use crate::db;
use crate::db::element::schema::Element;
use crate::log::RequestExtension;
use crate::service::overpass::OverpassElement;
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
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetArgs {
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    updated_since: Option<OffsetDateTime>,
    limit: Option<i64>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct GetItem {
    pub id: String,
    pub osm_json: OverpassElement,
    pub tags: Map<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub deleted_at: String,
}

impl From<Element> for GetItem {
    fn from(val: Element) -> GetItem {
        GetItem {
            id: val.overpass_data.btcmap_id(),
            osm_json: val.overpass_data,
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

impl From<Element> for Json<GetItem> {
    fn from(val: Element) -> Json<GetItem> {
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
            Redirect::to("https://static.btcmap.org/api/v2/elements.json").permanent(),
        ));
    }
    let elements = pool
        .get()
        .await?
        .interact(move |conn| match &args.updated_since {
            Some(updated_since) => {
                db::element::queries::select_updated_since(*updated_since, args.limit, true, conn)
            }
            None => db::element::queries::select_updated_since(
                OffsetDateTime::UNIX_EPOCH,
                args.limit,
                true,
                conn,
            ),
        })
        .await??;
    let elements_len = elements.len();
    let res = Either::Left(Json(elements.into_iter().map(|it| it.into()).collect()));
    req.extensions_mut()
        .insert(RequestExtension::new(elements_len));
    Ok(res)
}

#[get("{id}")]
pub async fn get_by_id(id: Path<String>, pool: Data<Pool>) -> Result<Json<GetItem>, Error> {
    let id = id.into_inner();
    let id_parts: Vec<String> = id.split(":").map(|it| it.into()).collect();
    let r#type = id_parts[0].clone();
    let id = id_parts[1].parse::<i64>().map_err(|_| "Invalid ID")?;

    pool.get()
        .await?
        .interact(move |conn| db::element::queries::select_by_osm_type_and_id(&r#type, id, conn))
        .await?
        .map(Into::into)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::mock_pool;
    use crate::{db, Result};
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};
    use time::macros::datetime;

    #[test]
    async fn get_empty_table() -> Result<()> {
        let pool = mock_pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
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
        let pool = mock_pool();
        let element = db::element::queries_async::insert(OverpassElement::mock(1), &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=100").to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], element.into());
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let pool = mock_pool();
        db::element::queries_async::insert(OverpassElement::mock(1), &pool).await?;
        db::element::queries_async::insert(OverpassElement::mock(2), &pool).await?;
        db::element::queries_async::insert(OverpassElement::mock(3), &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=2").to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 2);
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let pool = mock_pool();
        let element_1 = db::element::queries_async::insert(OverpassElement::mock(1), &pool).await?;
        db::element::queries_async::set_updated_at(
            element_1.id,
            datetime!(2022-01-05 00:00 UTC),
            &pool,
        )
        .await?;
        let element_2 = db::element::queries_async::insert(OverpassElement::mock(2), &pool).await?;
        db::element::queries_async::set_updated_at(
            element_2.id,
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
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 1);
        Ok(())
    }

    #[test]
    async fn get_by_id() -> Result<()> {
        let pool = mock_pool();
        let element = db::element::queries_async::insert(OverpassElement::mock(1), &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_id),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}", element.overpass_data.btcmap_id()))
            .to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res, element.into());
        Ok(())
    }
}
