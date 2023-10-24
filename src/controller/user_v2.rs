use crate::model::user;
use crate::model::OsmUserJson;
use crate::model::User;
use crate::service::auth::get_admin_token;
use crate::ApiError;
use actix_web::get;
use actix_web::patch;
use actix_web::web::Data;
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
    pub id: i32,
    pub osm_json: OsmUserJson,
    pub tags: Map<String, Value>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

impl Into<GetItem> for User {
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

#[get("")]
async fn get(args: Query<GetArgs>, db: Data<Connection>) -> Result<Json<Vec<GetItem>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => db
            .prepare(user::SELECT_UPDATED_SINCE)?
            .query_map(
                named_params! {
                   ":updated_since": updated_since,
                   ":limit": args.limit.unwrap_or(std::i32::MAX),
                },
                user::SELECT_UPDATED_SINCE_MAPPER,
            )?
            .map(|it| it.map(|it| it.into()))
            .collect::<Result<_, _>>()?,
        None => db
            .prepare(user::SELECT_ALL)?
            .query_map(
                named_params! { ":limit": args.limit.unwrap_or(std::i32::MAX) },
                user::SELECT_ALL_MAPPER,
            )?
            .map(|it| it.map(|it| it.into()))
            .collect::<Result<_, _>>()?,
    }))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<String>, db: Data<Connection>) -> Result<Json<GetItem>, ApiError> {
    let id = id.into_inner();

    db.query_row(
        user::SELECT_BY_ID,
        &[(":id", &id)],
        user::SELECT_BY_ID_MAPPER,
    )
    .optional()?
    .map(|it| Json(it.into()))
    .ok_or(ApiError::new(
        404,
        &format!("User with id {id} doesn't exist"),
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
    let user_id = id.into_inner();

    let keys: Vec<String> = args.keys().map(|it| it.to_string()).collect();

    warn!(
        actor_id = token.user_id,
        user_id,
        tags = keys.join(", "),
        "User attempted to update user tags",
    );

    let user: Option<User> = db
        .query_row(
            user::SELECT_BY_ID,
            named_params! { ":id": user_id },
            user::SELECT_BY_ID_MAPPER,
        )
        .optional()?;

    let user = match user {
        Some(v) => v,
        None => {
            return Err(ApiError::new(
                404,
                &format!("There is no user with id {user_id}"),
            ));
        }
    };

    let mut old_tags = user.tags.clone();

    let mut merged_tags = Map::new();
    merged_tags.append(&mut old_tags);
    merged_tags.append(&mut args.clone());

    db.execute(
        user::UPDATE_TAGS,
        named_params! {
            ":user_id": user_id,
            ":tags": serde_json::to_string(&merged_tags).unwrap(),
        },
    )?;

    Ok(HttpResponse::Ok())
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
    use serde_json::{json, Value};

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
            user::INSERT,
            named_params! {
                ":id": 1,
                ":osm_json": serde_json::to_string(&OsmUserJson::mock())?,
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
    async fn get_updated_since() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        conn.execute(
            "INSERT INTO user (id, osm_json, updated_at) VALUES (1, ?, '2022-01-05')",
            [serde_json::to_string(&OsmUserJson::mock())?],
        )?;
        conn.execute(
            "INSERT INTO user (id, osm_json, updated_at) VALUES (2, ?, '2022-02-05')",
            [serde_json::to_string(&OsmUserJson::mock())?],
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

        let user_id = 1;
        conn.execute(
            user::INSERT,
            named_params! {
                ":id": user_id,
                ":osm_json": serde_json::to_string(&OsmUserJson::mock())?,
            },
        )?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::get_by_id),
        )
        .await;
        let req = TestRequest::get().uri(&format!("/{user_id}")).to_request();
        let res: GetItem = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.id, user_id);

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

        let user_id = 1;
        conn.execute(
            user::INSERT,
            named_params! {
                ":id": user_id,
                ":osm_json": serde_json::to_string(&OsmUserJson::mock())?,
            },
        )?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(conn))
                .service(super::patch_tags),
        )
        .await;

        let req = TestRequest::patch()
            .uri(&format!("/{user_id}/tags"))
            .append_header(("Authorization", format!("Bearer {admin_token}")))
            .set_json(json!({ "foo": "bar" }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);

        Ok(())
    }
}
