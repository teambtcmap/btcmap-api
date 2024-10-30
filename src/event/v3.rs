use super::Event;
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
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
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
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, Value>>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl Into<GetItem> for Event {
    fn into(self) -> GetItem {
        let user_id = if self.deleted_at.is_none() {
            Some(self.user_id)
        } else {
            None
        };
        let element_id = if self.deleted_at.is_none() {
            Some(self.element_id)
        } else {
            None
        };
        let r#type = if self.deleted_at.is_none() {
            Some(match self.r#type.as_str() {
                "create" => 1,
                "update" => 2,
                "delete" => 3,
                _ => -1,
            })
        } else {
            None
        };
        let tags = if self.deleted_at.is_none() && !self.tags.is_empty() {
            Some(self.tags)
        } else {
            None
        };
        let created_at = if self.deleted_at.is_none() {
            Some(self.created_at)
        } else {
            None
        };
        GetItem {
            id: self.id,
            user_id,
            element_id,
            r#type,
            tags,
            created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

impl Into<Json<GetItem>> for Event {
    fn into(self) -> Json<GetItem> {
        Json(self.into())
    }
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Json<Vec<GetItem>>, Error> {
    let events = pool
        .get()
        .await?
        .interact(move |conn| match args.updated_since {
            Some(updated_since) => {
                Event::select_updated_since(&updated_since, Some(args.limit.unwrap_or(100)), conn)
            }
            None => Event::select_all(Some("DESC".into()), Some(args.limit.unwrap_or(100)), conn),
        })
        .await??;
    req.extensions_mut()
        .insert(RequestExtension::new("v3/events", events.len() as i64));
    Ok(Json(events.into_iter().map(|it| it.into()).collect()))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Arc<Pool>>) -> Result<Json<GetItem>, Error> {
    let id = id.into_inner();
    pool.get()
        .await?
        .interact(move |conn| Event::select_by_id(id, conn))
        .await??
        .map(|it| it.into())
        .ok_or(Error::NotFound(format!(
            "Event with id = {id} doesn't exist"
        )))
}

#[cfg(test)]
mod test {
    use crate::element::Element;
    use crate::event::Event;
    use crate::osm::osm::OsmUser;
    use crate::osm::overpass::OverpassElement;
    use crate::test::mock_state;
    use crate::user::User;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use time::macros::datetime;

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
        let user = User::insert(1, &OsmUser::mock(), &state.conn)?;
        let element = Element::insert(&OverpassElement::mock(1), &state.conn)?;
        let event = Event::insert(user.id, element.id, "", &state.conn)?;
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
        assert_eq!(res, vec![event.into()]);
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let state = mock_state().await;
        let user = User::insert(1, &OsmUser::mock(), &state.conn)?;
        let element = Element::insert(&OverpassElement::mock(1), &state.conn)?;
        let event_1 = Event::insert(user.id, element.id, "", &state.conn)?;
        let event_2 = Event::insert(user.id, element.id, "", &state.conn)?;
        let _event_3 = Event::insert(user.id, element.id, "", &state.conn)?;
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
        assert_eq!(res, vec![event_1.into(), event_2.into()]);
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let state = mock_state().await;
        let user = User::insert(1, &OsmUser::mock(), &state.conn)?;
        let element = Element::insert(&OverpassElement::mock(1), &state.conn)?;
        let event_1 = Event::insert(user.id, element.id, "", &state.conn)?;
        Event::set_updated_at(event_1.id, &datetime!(2022-01-05 00:00 UTC), &state.conn)?;
        let event_2 = Event::insert(user.id, element.id, "", &state.conn)?;
        let event_2 =
            Event::set_updated_at(event_2.id, &datetime!(2022-02-05 00:00 UTC), &state.conn)?;
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
        assert_eq!(res, vec![event_2.into()]);
        Ok(())
    }
}
