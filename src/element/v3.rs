use crate::element::Element;
use crate::element::ElementRepo;
use crate::osm::overpass::OverpassElement;
use crate::ApiError;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Query;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
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
pub async fn get(
    args: Query<GetArgs>,
    repo: Data<ElementRepo>,
) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(
        repo.select_updated_since(&args.updated_since, Some(args.limit))
            .await?
            .into_iter()
            .map(|it| it.into())
            .collect(),
    ))
}

#[cfg(test)]
mod test {
    use crate::element::ElementRepo;
    use crate::osm::overpass::OverpassElement;
    use crate::test::mock_state;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use http::StatusCode;
    use time::macros::datetime;

    #[test]
    async fn get_no_updated_since() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(ElementRepo::mock()))
                .service(scope("/limit=1").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(StatusCode::BAD_REQUEST, res.status());
        Ok(())
    }

    #[test]
    async fn get_empty_array() -> Result<()> {
        let state = mock_state().await;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.element_repo))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 0);
        Ok(())
    }

    #[test]
    async fn get_not_empty_array() -> Result<()> {
        let state = mock_state().await;
        let element = state.element_repo.insert(&OverpassElement::mock(1)).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.element_repo))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res, vec![element.into()]);
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let state = mock_state().await;
        let element_1 = state.element_repo.insert(&OverpassElement::mock(1)).await?;
        let element_2 = state.element_repo.insert(&OverpassElement::mock(2)).await?;
        let _element_3 = state.element_repo.insert(&OverpassElement::mock(3)).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.element_repo))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/?limit=2").to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res, vec![element_1.into(), element_2.into()]);
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let state = mock_state().await;
        let element_1 = state.element_repo.insert(&OverpassElement::mock(1)).await?;
        state
            .element_repo
            .set_updated_at(element_1.id, &datetime!(2022-01-05 00:00 UTC))
            .await?;
        let element_2 = state.element_repo.insert(&OverpassElement::mock(2)).await?;
        state
            .element_repo
            .set_updated_at(element_2.id, &datetime!(2022-02-05 00:00 UTC))
            .await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.element_repo))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2022-01-10T00:00:00Z")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res, vec![element_2.into()]);
        Ok(())
    }
}
