use crate::db;
use crate::db::main::element_issue::schema::ElementIssue;
use crate::db::main::element_issue::schema::SelectOrderedBySeverityRow;
use crate::db::main::MainPool;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult as Res;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Path;
use actix_web::web::Query;
use serde::Deserialize;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetArgs {
    area_id: i64,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
pub struct PlaceIssuesRes {
    pub total_issues: i64,
    pub requested_issues: Vec<ResItem>,
}

#[derive(Serialize)]
pub struct ResItem {
    pub element_osm_type: String,
    pub element_osm_id: i64,
    pub element_name: String,
    pub issue_code: String,
}

impl From<SelectOrderedBySeverityRow> for ResItem {
    fn from(val: SelectOrderedBySeverityRow) -> Self {
        ResItem {
            element_osm_type: val.element_osm_type,
            element_osm_id: val.element_osm_id,
            element_name: val.element_name.unwrap_or_default(),
            issue_code: val.issue_code,
        }
    }
}

#[derive(Serialize)]
pub struct Issue {
    pub id: i64,
    pub place_id: i64,
    pub code: String,
    pub severity: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<ElementIssue> for Issue {
    fn from(val: ElementIssue) -> Self {
        Issue {
            id: val.id,
            place_id: val.element_id,
            code: val.code,
            severity: val.severity,
            created_at: val.created_at,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

impl From<ElementIssue> for actix_web::web::Json<Issue> {
    fn from(val: ElementIssue) -> Self {
        actix_web::web::Json(val.into())
    }
}

#[get("")]
pub async fn get(args: Query<GetArgs>, pool: Data<MainPool>) -> Res<PlaceIssuesRes> {
    let total_issues = db::main::element_issue::queries::select_count(
        args.area_id,
        false,
        args.area_id != 662,
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;
    let requested_issues = db::main::element_issue::queries::select_ordered_by_severity(
        args.area_id,
        args.limit.unwrap_or(50),
        args.offset.unwrap_or(0),
        args.area_id != 662,
        &pool,
    )
    .await
    .map_err(|_| RestApiError::database())?;
    let response = PlaceIssuesRes {
        total_issues,
        requested_issues: requested_issues.into_iter().map(Into::into).collect(),
    };
    Ok(actix_web::web::Json(response))
}

#[get("{id}")]
pub async fn get_by_id(id: Path<i64>, pool: Data<MainPool>) -> Res<Issue> {
    db::main::element_issue::queries::select_by_id(*id, &pool)
        .await
        .map_err(|_| RestApiError::database())
        .map(Into::into)
}
