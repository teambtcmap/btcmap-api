use crate::model::element;
use crate::model::Element;
use crate::service::auth::is_from_admin;
use crate::ApiError;
use actix_web::get;
use actix_web::post;
use actix_web::web::Data;
use actix_web::web::Form;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::Responder;
use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct GetArgs {
    updated_since: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GetItem {
    pub id: String,
    pub osm_json: Value,
    pub tags: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

impl Into<GetItem> for Element {
    fn into(self) -> GetItem {
        GetItem {
            id: self.id,
            osm_json: self.osm_json,
            tags: self.tags,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PostTagsArgs {
    name: String,
    value: String,
}

#[get("")]
pub async fn get(
    args: Query<GetArgs>,
    db: Data<Connection>,
) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => db
            .prepare(element::SELECT_UPDATED_SINCE)?
            .query_map(
                &[(":updated_since", updated_since)],
                element::SELECT_UPDATED_SINCE_MAPPER,
            )?
            .map(|it| it.map(|it| it.into()))
            .collect::<Result<_, _>>()?,
        None => db
            .prepare(element::SELECT_ALL)?
            .query_map([], element::SELECT_ALL_MAPPER)?
            .map(|it| it.map(|it| it.into()))
            .collect::<Result<_, _>>()?,
    }))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<String>, db: Data<Connection>) -> Result<Json<GetItem>, ApiError> {
    let id = id.into_inner();

    db.query_row(
        element::SELECT_BY_ID,
        &[(":id", &id)],
        element::SELECT_BY_ID_MAPPER,
    )
    .optional()?
    .map(|it| Json(it.into()))
    .ok_or(ApiError::new(
        404,
        &format!("Element with id {id} doesn't exist"),
    ))
}

#[post("{id}/tags")]
async fn post_tags(
    id: Path<String>,
    req: HttpRequest,
    args: Form<PostTagsArgs>,
    db: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    is_from_admin(&req)?;

    let id = id.into_inner();

    let element: Option<Element> = db
        .query_row(
            element::SELECT_BY_ID,
            &[(":id", &id)],
            element::SELECT_BY_ID_MAPPER,
        )
        .optional()?;

    match element {
        Some(element) => {
            if args.value.len() > 0 {
                db.execute(
                    element::INSERT_TAG,
                    named_params! {
                        ":element_id": element.id,
                        ":tag_name": format!("$.{}", args.name),
                        ":tag_value": args.value,
                    },
                )?;
            } else {
                db.execute(
                    element::DELETE_TAG,
                    named_params! {
                        ":element_id": element.id,
                        ":tag_name": format!("$.{}", args.name),
                    },
                )?;
            }

            Ok(HttpResponse::Created())
        }
        None => Err(ApiError::new(
            404,
            &format!("There is no element with id {id}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::db::tests::db;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};
    use reqwest::StatusCode;
    use rusqlite::named_params;
    use std::env;

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
            element::INSERT,
            named_params! {
                ":id": "node:1",
                ":osm_json": "{}",
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
            "INSERT INTO element (id, osm_json, updated_at) VALUES ('node:1', '{}', '2022-01-05')",
            [],
        )?;
        db.execute(
            "INSERT INTO element (id, osm_json, updated_at) VALUES ('node:2', '{}', '2022-02-05')",
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
        let element_id = "node:1";
        db.execute(
            element::INSERT,
            named_params! {
                ":id": element_id,
                ":osm_json": "{}",
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

    #[actix_web::test]
    async fn post_tags() -> Result<()> {
        let admin_token = "test";
        env::set_var("ADMIN_TOKEN", admin_token);
        let db = db()?;
        let element_id = "node:1";
        db.execute(
            element::INSERT,
            named_params! {
                ":id": element_id,
                ":osm_json": "{}",
            },
        )?;
        let app =
            test::init_service(App::new().app_data(Data::new(db)).service(super::post_tags)).await;
        let req = TestRequest::post()
            .uri(&format!("/{element_id}/tags"))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_form(PostTagsArgs {
                name: "foo".into(),
                value: "bar".into(),
            })
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::CREATED);
        Ok(())
    }
}
