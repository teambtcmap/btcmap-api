use super::ElementComment;
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
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetArgs {
    #[serde(with = "time::serde::rfc3339")]
    updated_since: OffsetDateTime,
    limit: i64,
}

#[derive(Serialize)]
pub struct GetItem {
    pub id: i64,
    pub element_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub created_at: Option<OffsetDateTime>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl Into<GetItem> for ElementComment {
    fn into(self) -> GetItem {
        let element_id = if self.deleted_at.is_none() {
            Some(self.element_id)
        } else {
            None
        };
        let created_at = if self.deleted_at.is_none() {
            Some(self.created_at)
        } else {
            None
        };
        let comment = if self.deleted_at.is_none() {
            Some(self.comment)
        } else {
            None
        };
        GetItem {
            id: self.id,
            element_id,
            comment,
            created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

impl Into<Json<GetItem>> for ElementComment {
    fn into(self) -> Json<GetItem> {
        Json(self.into())
    }
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetArgs>,
    pool: Data<Pool>,
) -> Result<Json<Vec<GetItem>>, Error> {
    let element_comments = pool
        .get()
        .await?
        .interact(move |conn| {
            ElementComment::select_updated_since(&args.updated_since, Some(args.limit), conn)
        })
        .await??;
    req.extensions_mut().insert(RequestExtension::new(
        "v3/element-comments",
        element_comments.len() as i64,
    ));
    Ok(Json(
        element_comments.into_iter().map(|it| it.into()).collect(),
    ))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<Pool>) -> Result<Json<GetItem>, Error> {
    let id_clone = id.clone();
    pool.get()
        .await?
        .interact(move |conn| ElementComment::select_by_id(id_clone, conn))
        .await??
        .ok_or(Error::NotFound(format!(
            "Element comment with id {id} doesn't exist"
        )))
        .map(|it| it.into())
}
