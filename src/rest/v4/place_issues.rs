use crate::db;
use crate::db::main::element_issue::schema::SelectOrderedBySeverityRow;
use crate::db::main::MainPool;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult as Res;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Query;
use serde::Deserialize;
use serde::Serialize;

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
