use crate::db;
use crate::db::element_comment::schema::ElementComment;
use crate::log::RequestExtension;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult as Res;
use crate::service;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::HttpMessage;
use actix_web::HttpRequest;
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use serde::Deserialize;
use serde::Serialize;
use std::i64;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetListArgs {
    fields: Option<String>,
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    updated_since: Option<OffsetDateTime>,
    limit: Option<i64>,
    include_deleted: Option<bool>,
}

#[derive(Deserialize)]
pub struct GetSingleArgs {
    fields: Option<String>,
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetListArgs>,
    pool: Data<Pool>,
) -> Res<Vec<JsonObject>> {
    let fields: Vec<&str> = args.fields.as_deref().unwrap_or("").split(',').collect();
    let updated_since = args.updated_since.unwrap_or(OffsetDateTime::UNIX_EPOCH);
    let include_deleted = args.include_deleted.unwrap_or(false);

    let items = db::element::queries::select_updated_since(
        updated_since,
        args.limit,
        include_deleted,
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;

    req.extensions_mut()
        .insert(RequestExtension::new(items.len()));

    let items = items
        .into_iter()
        .map(|it| service::element::generate_tags(&it, &fields))
        .collect();

    Ok(Json(items))
}

#[derive(Deserialize)]
pub struct GetBoostedArgs {
    fields: Option<String>,
}

#[get("/boosted")]
pub async fn get_boosted(
    req: HttpRequest,
    args: Query<GetBoostedArgs>,
    pool: Data<Pool>,
) -> Res<Vec<JsonObject>> {
    let fields: Vec<&str> = args.fields.as_deref().unwrap_or("").split(',').collect();
    let updated_since = OffsetDateTime::UNIX_EPOCH;
    let include_deleted = false;

    let items =
        db::element::queries::select_updated_since(updated_since, None, include_deleted, &pool)
            .await
            .map_err(|_| RestApiError::database())?;

    req.extensions_mut()
        .insert(RequestExtension::new(items.len()));

    let items = items
        .into_iter()
        .filter(|it| match it.tags.get("boost:expires") {
            Some(boost_expires) => match boost_expires.as_str() {
                Some(boost_expires) => match OffsetDateTime::parse(boost_expires, &Rfc3339) {
                    Ok(boost_expires) => boost_expires > OffsetDateTime::now_utc(),
                    Err(_) => false,
                },
                None => false,
            },
            None => false,
        })
        .map(|it| service::element::generate_tags(&it, &fields))
        .collect();

    Ok(Json(items))
}

#[get("{id}")]
pub async fn get_by_id(
    id: Path<String>,
    args: Query<GetSingleArgs>,
    pool: Data<Pool>,
) -> Res<JsonObject> {
    let fields: Vec<&str> = args.fields.as_deref().unwrap_or("").split(',').collect();
    db::element::queries::select_by_id_or_osm_id(id.into_inner(), &pool)
        .await
        .map(|it| Json(service::element::generate_tags(&it, &fields)))
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })
}

#[derive(Serialize)]
pub struct Comment {
    pub id: i64,
    pub text: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl From<ElementComment> for Comment {
    fn from(val: ElementComment) -> Self {
        Comment {
            id: val.id,
            text: val.comment,
            created_at: val.created_at,
        }
    }
}

#[get("{id}/comments")]
pub async fn get_by_id_comments(id: Path<String>, pool: Data<Pool>) -> Res<Vec<Comment>> {
    let element = db::element::queries::select_by_id_or_osm_id(id.as_str(), &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => RestApiError::not_found(),
            _ => RestApiError::database(),
        })?;
    db::element_comment::queries::select_by_element_id(element.id, false, i64::MAX, &pool)
        .await
        .map(|it| Json(it.into_iter().map(Comment::from).collect()))
        .map_err(|_| RestApiError::database())
}

#[cfg(test)]
mod test {
    use crate::db::test::pool;
    use crate::service::overpass::OverpassElement;
    use crate::{db, Result};
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use geojson::JsonObject;
    use serde_json::{Map, Value};
    use time::macros::datetime;

    #[test]
    async fn get_empty_array() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool()))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    async fn get_not_empty_array() -> Result<()> {
        let pool = pool();
        let element = db::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!(element.id, res.first().unwrap()["id"].as_i64().unwrap());
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let pool = pool();
        let _element_1 = db::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let _element_2 = db::element::queries::insert(OverpassElement::mock(2), &pool).await?;
        let _element_3 = db::element::queries::insert(OverpassElement::mock(3), &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=2").to_request();
        let res: Vec<JsonObject> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, res.len());
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let pool = pool();
        let element_1 = db::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        db::element::queries::set_updated_at(element_1.id, datetime!(2022-01-05 00:00 UTC), &pool)
            .await?;
        let element_2 = db::element::queries::insert(OverpassElement::mock(2), &pool).await?;
        let _element_2 = db::element::queries::set_updated_at(
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
        let res: Vec<Map<String, Value>> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        Ok(())
    }

    #[test]
    async fn get_by_id() -> Result<()> {
        let pool = pool();
        let element = db::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .service(super::get_by_id),
        )
        .await;
        let req = TestRequest::get().uri("/1").to_request();
        let res: JsonObject = test::call_and_read_body_json(&app, req).await;
        assert_eq!(element.id, res["id"].as_i64().unwrap());
        Ok(())
    }
}
