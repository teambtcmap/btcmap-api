use crate::{
    db::{self, element_issue::schema::SelectOrderedBySeverityRow},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub area_id: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Serialize)]
pub struct Res {
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

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let total_issues =
        db::element_issue::queries::select_count(params.area_id, false, pool).await?;
    let requested_issues = db::element_issue::queries::select_ordered_by_severity(
        params.area_id,
        params.limit,
        params.offset,
        pool,
    )
    .await?;
    Ok(Res {
        total_issues,
        requested_issues: requested_issues.into_iter().map(Into::into).collect(),
    })
}
