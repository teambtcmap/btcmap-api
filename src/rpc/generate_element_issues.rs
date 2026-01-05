use crate::{
    db::{self},
    service::{self},
    Result,
};
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Res {
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub finished_at: OffsetDateTime,
    pub time_s: f64,
    pub affected_elements: i64,
}

pub async fn run(pool: &Pool) -> Result<Res> {
    let elements =
        db::element::queries::select_updated_since(OffsetDateTime::UNIX_EPOCH, None, true, pool)
            .await?;
    for element in elements {
        if element.deleted_at.is_some() {
            let issues = db::element_issue::queries::select_by_element_id(element.id, pool).await?;
            for issue in issues {
                db::element_issue::queries::set_deleted_at(
                    issue.id,
                    Some(OffsetDateTime::now_utc()),
                    pool,
                )
                .await?;
            }
        }
    }
    let elements =
        db::element::queries::select_updated_since(OffsetDateTime::UNIX_EPOCH, None, false, pool)
            .await?;
    let res = service::element::generate_issues(elements.iter().collect(), pool).await?;
    Ok(Res {
        started_at: res.started_at,
        finished_at: res.finished_at,
        time_s: res.time_s,
        affected_elements: res.affected_elements,
    })
}
