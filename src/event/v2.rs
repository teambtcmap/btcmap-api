use super::Event;
use crate::event::model::EventRepo;
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
use time::Duration;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetArgs {
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    updated_since: Option<OffsetDateTime>,
    limit: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct GetItem {
    pub id: i64,
    pub user_id: i64,
    pub element_id: String,
    pub r#type: String,
    pub tags: HashMap<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub deleted_at: String,
}

impl Into<GetItem> for Event {
    fn into(self) -> GetItem {
        GetItem {
            id: self.id,
            user_id: self.user_id,
            element_id: format!("{}:{}", self.element_osm_type, self.element_osm_id),
            r#type: self.r#type,
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

impl Into<Json<GetItem>> for Event {
    fn into(self) -> Json<GetItem> {
        Json(self.into())
    }
}

#[get("")]
async fn get(args: Query<GetArgs>, repo: Data<EventRepo>) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => repo
            .select_updated_since(updated_since, args.limit)
            .await?
            .into_iter()
            .map(|it| it.into())
            .collect(),
        None => repo
            .select_updated_since(
                &OffsetDateTime::now_utc()
                    .checked_sub(Duration::days(30))
                    .unwrap(),
                args.limit,
            )
            .await?
            .into_iter()
            .map(|it| it.into())
            .collect(),
    }))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, repo: Data<EventRepo>) -> Result<Json<GetItem>, ApiError> {
    let id = id.into_inner();
    repo.select_by_id(id)
        .await?
        .map(|it| it.into())
        .ok_or(ApiError::new(
            StatusCode::NOT_FOUND,
            &format!("Event with id = {id} doesn't exist"),
        ))
}

#[cfg(test)]
mod test {
    use crate::event::v2::GetItem;
    use crate::osm::overpass::OverpassElement;
    use crate::test::mock_state;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use serde_json::Value;

    #[test]
    async fn get_empty_table() -> Result<()> {
        let state = mock_state();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.event_repo))
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
        state.event_repo.insert(1, element.id, "").await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.event_repo))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 1);
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let state = mock_state();
        state.element_repo.insert(&OverpassElement::mock(1)).await?;
        state.event_repo.insert(1, 1, "").await?;
        state.event_repo.insert(1, 1, "").await?;
        state.event_repo.insert(1, 1, "").await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.event_repo))
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
        let state = mock_state();
        state.element_repo.insert(&OverpassElement::mock(1)).await?;
        state.conn.execute(
            "INSERT INTO event (element_id, type, user_id, updated_at) VALUES (1, '', 0, '2022-01-05T00:00:00Z')",
            [],
        )?;
        state.conn.execute(
            "INSERT INTO event (element_id, type, user_id, updated_at) VALUES (1, '', 0, '2022-02-05T00:00:00Z')",
            [],
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.event_repo))
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
    async fn get_by_id() -> Result<()> {
        let state = mock_state();
        let event_id = 1;
        state.element_repo.insert(&OverpassElement::mock(1)).await?;
        state.event_repo.insert(1, 1, "").await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(state.event_repo))
                .service(super::get_by_id),
        )
        .await;
        let req = TestRequest::get().uri(&format!("/{event_id}")).to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, event_id);
        Ok(())
    }
}
