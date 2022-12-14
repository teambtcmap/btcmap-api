use crate::model::event;
use crate::model::Event;
use crate::ApiError;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize)]
pub struct GetArgs {
    updated_since: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GetItem {
    pub id: i64,
    pub r#type: String,
    pub element_id: String,
    pub user_id: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

impl Into<GetItem> for Event {
    fn into(self) -> GetItem {
        GetItem {
            id: self.id,
            r#type: self.r#type,
            element_id: self.element_id,
            user_id: self.user_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

#[get("")]
async fn get(args: Query<GetArgs>, db: Data<Connection>) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => db
            .prepare(event::SELECT_UPDATED_SINCE)?
            .query_map(
                &[(":updated_since", updated_since)],
                event::SELECT_UPDATED_SINCE_MAPPER,
            )?
            .map(|it| it.map(|it| it.into()))
            .collect::<Result<_, _>>()?,
        None => db
            .prepare(event::SELECT_ALL)?
            .query_map([], event::SELECT_ALL_MAPPER)?
            .map(|it| it.map(|it| it.into()))
            .collect::<Result<_, _>>()?,
    }))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<String>, db: Data<Connection>) -> Result<Json<GetItem>, ApiError> {
    let id = id.into_inner();

    db.query_row(
        event::SELECT_BY_ID,
        &[(":id", &id)],
        event::SELECT_BY_ID_MAPPER,
    )
    .optional()?
    .map(|it| Json(it.into()))
    .ok_or(ApiError::new(
        404,
        &format!("Event with id {id} doesn't exist"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::db::tests::db;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};
    use rusqlite::named_params;
    use serde_json::Value;

    #[actix_web::test]
    async fn get_empty_table() -> Result<()> {
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db()?))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 0);
        Ok(())
    }

    #[actix_web::test]
    async fn get_one_row() -> Result<()> {
        let db = db()?;
        db.execute(
            event::INSERT,
            named_params! {
                ":user_id": "0",
                ":element_id": "",
                ":type": "",
            },
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 1);
        Ok(())
    }

    #[actix_web::test]
    async fn get_updated_since() -> Result<()> {
        let db = db()?;
        db.execute(
            "INSERT INTO event (element_id, type, user_id, updated_at) VALUES ('', '', 0, '2022-01-05')",
            [],
        )?;
        db.execute(
            "INSERT INTO event (element_id, type, user_id, updated_at) VALUES ('', '', 0, '2022-02-05')",
            [],
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2022-01-10")
            .to_request();
        let res: Vec<GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 1);
        Ok(())
    }

    #[actix_web::test]
    async fn get_by_id() -> Result<()> {
        let db = db()?;
        let element_id = 1;
        db.execute(
            event::INSERT,
            named_params! {
                ":user_id": "0",
                ":element_id": "",
                ":type": "",
            },
        )?;
        let app =
            test::init_service(App::new().app_data(Data::new(db)).service(super::get_by_id)).await;
        let req = TestRequest::get()
            .uri(&format!("/{element_id}"))
            .to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, element_id);
        Ok(())
    }
}
