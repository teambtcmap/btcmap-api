use crate::element::Element;
use crate::osm::overpass::OverpassElement;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
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
    pub osm_data: Option<OverpassElement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, Value>>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl Into<GetItem> for Element {
    fn into(self) -> GetItem {
        let overpass_data = if self.deleted_at.is_none() {
            Some(self.overpass_data)
        } else {
            None
        };
        let tags = if self.deleted_at.is_none() {
            Some(self.tags)
        } else {
            None
        };
        GetItem {
            id: self.id,
            osm_data: overpass_data,
            tags,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

impl Into<Json<GetItem>> for Element {
    fn into(self) -> Json<GetItem> {
        Json(self.into())
    }
}

#[get("")]
pub async fn get(args: Query<GetArgs>, pool: Data<Arc<Pool>>) -> Result<Json<Vec<GetItem>>, Error> {
    let elements = pool
        .get()
        .await?
        .interact(move |conn| {
            Element::select_updated_since(&args.updated_since, Some(args.limit), conn)
        })
        .await??;
    Ok(Json(elements.into_iter().map(|it| it.into()).collect()))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<String>, pool: Data<Arc<Pool>>) -> Result<Json<GetItem>, Error> {
    let id_clone = id.clone();
    pool.get()
        .await?
        .interact(move |conn| Element::select_by_id_or_osm_id(&id_clone, conn))
        .await??
        .ok_or(Error::HttpNotFound(format!(
            "Element with id {id} doesn't exist"
        )))
        .map(|it| it.into())
}

#[cfg(test)]
mod test {
    use crate::element::Element;
    use crate::error::{self, ApiError};
    use crate::osm::overpass::OverpassElement;
    use crate::test::mock_state;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data, QueryConfig};
    use actix_web::{test, App};
    use http::StatusCode;
    use time::macros::datetime;

    #[test]
    async fn get_no_updated_since() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(QueryConfig::default().error_handler(error::query_error_handler))
                .app_data(Data::new(mock_state().await.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=1").to_request();
        let res: ApiError = test::try_call_and_read_body_json(&app, req).await.unwrap();
        assert_eq!(StatusCode::BAD_REQUEST.as_u16(), res.http_code);
        assert!(res.message.contains("missing field `updated_since`"));
        Ok(())
    }

    #[test]
    async fn get_no_limit() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(QueryConfig::default().error_handler(error::query_error_handler))
                .app_data(mock_state().await.pool)
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z")
            .to_request();
        let res: ApiError = test::try_call_and_read_body_json(&app, req).await.unwrap();
        assert_eq!(StatusCode::BAD_REQUEST.as_u16(), res.http_code);
        assert!(res.message.contains("missing field `limit`"));
        Ok(())
    }

    #[test]
    async fn get_empty_array() -> Result<()> {
        let state = mock_state().await;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
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
        let state = mock_state().await;
        let element = Element::insert(&OverpassElement::mock(1), &state.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=1")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res, vec![element.into()]);
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let state = mock_state().await;
        let element_1 = Element::insert(&OverpassElement::mock(1), &state.conn)?;
        let element_2 = Element::insert(&OverpassElement::mock(2), &state.conn)?;
        let _element_3 = Element::insert(&OverpassElement::mock(3), &state.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=2")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res, vec![element_1.into(), element_2.into()]);
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let state = mock_state().await;
        let element_1 = Element::insert(&OverpassElement::mock(1), &state.conn)?;
        Element::_set_updated_at(element_1.id, &datetime!(2022-01-05 00:00 UTC), &state.conn)?;
        let element_2 = Element::insert(&OverpassElement::mock(2), &state.conn)?;
        let element_2 =
            Element::_set_updated_at(element_2.id, &datetime!(2022-02-05 00:00 UTC), &state.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2022-01-10T00:00:00Z&limit=100")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res, vec![element_2.into()]);
        Ok(())
    }
}
