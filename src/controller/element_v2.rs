use crate::model::element;
use crate::model::Element;
use crate::service::auth::get_admin_token;
use crate::ApiError;
use actix_web::get;
use actix_web::patch;
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
use serde_json::Map;
use serde_json::Value;
use tracing::warn;

#[derive(Deserialize)]
pub struct GetArgs {
    updated_since: Option<String>,
    limit: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct GetItem {
    pub id: String,
    pub osm_json: Value,
    pub tags: Map<String, Value>,
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
                named_params! {
                    ":updated_since": updated_since,
                    ":limit": args.limit.unwrap_or(std::i32::MAX),
                },
                element::SELECT_UPDATED_SINCE_MAPPER,
            )?
            .map(|it| it.map(|it| it.into()))
            .collect::<Result<_, _>>()?,
        None => db
            .prepare(element::SELECT_ALL)?
            .query_map(
                named_params! { ":limit": args.limit.unwrap_or(std::i32::MAX) },
                element::SELECT_ALL_MAPPER,
            )?
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

#[patch("{id}/tags")]
async fn patch_tags(
    args: Json<Map<String, Value>>,
    db: Data<Connection>,
    id: Path<String>,
    req: HttpRequest,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&db, &req)?;
    let element_id = id.into_inner();

    let keys: Vec<String> = args.keys().map(|it| it.to_string()).collect();

    warn!(
        user_id = token.user_id,
        element_id,
        tags = keys.join(", "),
        "User attempted to update element tags",
    );

    let element: Option<Element> = db
        .query_row(
            element::SELECT_BY_ID,
            named_params! { ":id": element_id },
            element::SELECT_BY_ID_MAPPER,
        )
        .optional()?;

    let element = match element {
        Some(v) => v,
        None => {
            return Err(ApiError::new(
                404,
                &format!("There is no element with id {element_id}"),
            ));
        }
    };

    let mut old_tags = element.tags.clone();

    let mut merged_tags = Map::new();
    merged_tags.append(&mut old_tags);
    merged_tags.append(&mut args.clone());

    db.execute(
        element::UPDATE_TAGS,
        named_params! {
            ":element_id": element_id,
            ":tags": serde_json::to_string(&merged_tags).unwrap(),
        },
    )?;

    Ok(HttpResponse::Ok())
}

#[post("{id}/tags")]
async fn post_tags(
    id: Path<String>,
    req: HttpRequest,
    args: Form<PostTagsArgs>,
    db: Data<Connection>,
) -> Result<impl Responder, ApiError> {
    let token = get_admin_token(&db, &req)?;
    let element_id = id.into_inner();

    warn!(
        deprecated_api = true,
        user_id = token.user_id,
        element_id,
        tag_name = args.name,
        tag_value = args.value,
        "User attempted to update element tag",
    );

    let element: Option<Element> = db
        .query_row(
            element::SELECT_BY_ID,
            named_params! { ":id": element_id },
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
            &format!("There is no element with id {element_id}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::db;
    use crate::model::token;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::scope;
    use actix_web::{test, App};
    use reqwest::StatusCode;
    use rusqlite::named_params;
    use serde_json::json;

    #[actix_web::test]
    async fn get_empty_table() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
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
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        conn.execute(
            element::INSERT,
            named_params! {
                ":id": "node:1",
                ":osm_json": "{}",
            },
        )?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(scope("/").service(super::get)),
        )
        .await;

        let req = TestRequest::get().uri("/").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 1);

        Ok(())
    }

    #[actix_web::test]
    async fn get_with_limit() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        conn.execute("INSERT INTO element (id, updated_at) VALUES ('node:1', '2023-05-05')", [])?;
        conn.execute("INSERT INTO element (id, updated_at) VALUES ('node:2', '2023-05-06')", [])?;
        conn.execute("INSERT INTO element (id, updated_at) VALUES ('node:3', '2023-05-07')", [])?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(scope("/").service(super::get)),
        )
        .await;

        let req = TestRequest::get().uri("/?limit=2").to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.as_array().unwrap().len(), 2);

        Ok(())
    }

    #[actix_web::test]
    async fn get_updated_since() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        conn.execute(
            "INSERT INTO element (id, osm_json, updated_at) VALUES ('node:1', '{}', '2022-01-05')",
            [],
        )?;
        conn.execute(
            "INSERT INTO element (id, osm_json, updated_at) VALUES ('node:2', '{}', '2022-02-05')",
            [],
        )?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
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
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        let element_id = "node:1";
        conn.execute(
            element::INSERT,
            named_params! {
                ":id": element_id,
                ":osm_json": "{}",
            },
        )?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::get_by_id),
        )
        .await;

        let req = TestRequest::get()
            .uri(&format!("/{element_id}"))
            .to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, element_id);

        Ok(())
    }

    #[actix_web::test]
    async fn patch_tags() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        let admin_token = "test";
        conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;

        let element_id = "node:1";
        conn.execute(
            element::INSERT,
            named_params! {
                ":id": element_id,
                ":osm_json": "{}",
            },
        )?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::patch_tags),
        )
        .await;

        let req = TestRequest::patch()
            .uri(&format!("/{element_id}/tags"))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_json(json!({ "foo": "bar" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);

        Ok(())
    }

    #[actix_web::test]
    async fn post_tags() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        let admin_token = "test";
        conn.execute(
            token::INSERT,
            named_params! { ":user_id": 1, ":secret": admin_token },
        )?;

        let element_id = "node:1";
        conn.execute(
            element::INSERT,
            named_params! {
                ":id": element_id,
                ":osm_json": "{}",
            },
        )?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::post_tags),
        )
        .await;

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
