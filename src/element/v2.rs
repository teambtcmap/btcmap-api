use crate::element::Element;
use crate::element::ElementRepo;
use crate::osm::overpass::OverpassElement;
use crate::ApiError;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use http::StatusCode;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
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
    pub tags: HashMap<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub deleted_at: String,
}

impl Into<GetItem> for Element {
    fn into(self) -> GetItem {
        GetItem {
            id: self.overpass_data.btcmap_id(),
            osm_json: self.overpass_data,
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
    Ok(Json(match &args.updated_since {
        Some(updated_since) => repo
            .select_updated_since(&updated_since, args.limit)
            .await?
            .into_iter()
            .map(|it| it.into())
            .collect(),
        None => repo
            .select_all(args.limit)
            .await?
            .into_iter()
            .map(|it| it.into())
            .collect(),
    }))
}

#[get("{id}")]
pub async fn get_by_osm_type_and_id(
    id: Path<String>,
    repo: Data<ElementRepo>,
) -> Result<Json<GetItem>, ApiError> {
    let id_parts: Vec<&str> = id.split(":").collect();
    let r#type = id_parts[0];
    let id = id_parts[1].parse::<i64>()?;
    repo.select_by_osm_type_and_id(r#type, id)
        .await?
        .map(|it| it.into())
        .ok_or(ApiError::new(
            StatusCode::NOT_FOUND,
            &format!("Element with id {id} doesn't exist"),
        ))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::mock_state;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};
    use time::macros::datetime;

    #[test]
    async fn get_empty_table() -> Result<()> {
        let state = mock_state();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.element_repo))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 0);
        Ok(())
    }

    #[test]
    async fn get_one_row() -> Result<()> {
        let state = mock_state();
        let element = state.element_repo.insert(&OverpassElement::mock(1)).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.element_repo))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], element.into());
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let state = mock_state();
        state.element_repo.insert(&OverpassElement::mock(1)).await?;
        state.element_repo.insert(&OverpassElement::mock(2)).await?;
        state.element_repo.insert(&OverpassElement::mock(3)).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.element_repo))
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
        let state = mock_state();
        state
            .element_repo
            .insert(&OverpassElement::mock(1))
            .await?
            .set_updated_at(&datetime!(2022-01-05 00:00 UTC), &state.conn)?;
        state
            .element_repo
            .insert(&OverpassElement::mock(2))
            .await?
            .set_updated_at(&datetime!(2022-02-05 00:00 UTC), &state.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.element_repo))
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

    #[test]
    async fn get_by_osm_type_and_id() -> Result<()> {
        let state = mock_state();
        let element = state.element_repo.insert(&OverpassElement::mock(1)).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.element_repo))
                .service(super::get_by_osm_type_and_id),
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
