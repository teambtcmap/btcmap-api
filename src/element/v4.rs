use crate::element::Element;
use crate::element_comment::ElementComment;
use crate::log::RequestExtension;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::HttpMessage;
use actix_web::HttpRequest;
use actix_web_lab::extract::Query;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use std::i64;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetListArgs {
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    updated_since: Option<OffsetDateTime>,
    limit: Option<i64>,
    include_deleted: Option<bool>,
    f: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct GetSingleArgs {
    include_tag: Option<Vec<String>>,
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetListArgs>,
    pool: Data<Pool>,
) -> Result<Json<Vec<Map<String, Value>>>, Error> {
    let include_tags = args.f.clone().unwrap_or(vec![]);
    let include_tags: Vec<_> = include_tags.iter().map(String::as_str).collect();
    let elements = pool
        .get()
        .await?
        .interact(move |conn| {
            Element::select_updated_since(
                &args
                    .updated_since
                    .unwrap_or(OffsetDateTime::parse("2020-01-01T00:00:00Z", &Rfc3339).unwrap()),
                Some(args.limit.unwrap_or(i64::MAX)),
                args.include_deleted.unwrap_or(false),
                conn,
            )
        })
        .await??;
    req.extensions_mut()
        .insert(RequestExtension::new(elements.len()));
    let items: Vec<Map<String, Value>> = elements
        .into_iter()
        .map(|it| super::service::generate_tags(&it, &include_tags))
        .collect();
    Ok(Json(items))
}

#[get("{id}")]
pub async fn get_by_id(
    id: Path<String>,
    args: Query<GetSingleArgs>,
    pool: Data<Pool>,
) -> Result<Json<Map<String, Value>>, Error> {
    let include_tags = args.include_tag.clone().unwrap_or(vec![]);
    let include_tags: Vec<_> = include_tags.iter().map(String::as_str).collect();
    let id_clone = id.clone();
    pool.get()
        .await?
        .interact(move |conn| Element::select_by_id_or_osm_id(&id_clone, conn))
        .await??
        .ok_or(Error::not_found())
        .map(|it| super::service::generate_tags(&it, &include_tags))
        .map(|it| Json(it))
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

impl From<ElementComment> for Json<Comment> {
    fn from(val: ElementComment) -> Self {
        Json(val.into())
    }
}

#[get("{id}/comments")]
pub async fn get_by_id_comments(
    id: Path<String>,
    pool: Data<Pool>,
) -> Result<Json<Vec<Comment>>, Error> {
    let element = Element::select_by_id_or_osm_id_async(id.clone(), &pool)
        .await?
        .ok_or(Error::not_found())?;
    let comments =
        ElementComment::select_by_element_id_async(element.id, false, i64::MAX, &pool).await?;
    Ok(Json(comments.into_iter().map(|it| it.into()).collect()))
}

#[cfg(test)]
mod test {
    use crate::element::Element;
    use crate::osm::overpass::OverpassElement;
    use crate::test::mock_db;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use serde_json::{Map, Value};
    use time::macros::datetime;

    #[test]
    async fn get_empty_array() -> Result<()> {
        let db = mock_db();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=1")
            .to_request();
        let res: Vec<Map<String, Value>> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 0);
        Ok(())
    }

    #[test]
    async fn get_not_empty_array() -> Result<()> {
        let db = mock_db();
        let _element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=1")
            .to_request();
        let res: Vec<Map<String, Value>> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let db = mock_db();
        let _element_1 = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        let _element_2 = Element::insert(&OverpassElement::mock(2), &db.conn)?;
        let _element_3 = Element::insert(&OverpassElement::mock(3), &db.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=2")
            .to_request();
        let res: Vec<Map<String, Value>> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, res.len());
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let db = mock_db();
        let element_1 = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        Element::_set_updated_at(element_1.id, &datetime!(2022-01-05 00:00 UTC), &db.conn)?;
        let element_2 = Element::insert(&OverpassElement::mock(2), &db.conn)?;
        let _element_2 =
            Element::_set_updated_at(element_2.id, &datetime!(2022-02-05 00:00 UTC), &db.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
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
}
