use crate::{
    db::{self},
    service, Result,
};
use deadpool_sqlite::Pool;
use serde::Serialize;
use serde_json::json;
use time::OffsetDateTime;
use tracing::info;

#[derive(Serialize)]
pub struct Res {
    issues_pending: i64,
    issues_created: i64,
    issues_closed: i64,
}

pub async fn run(pool: &Pool) -> Result<Res> {
    let submissions = db::place_submission::queries::select_open_and_not_revoked(pool).await?;
    info!(
        len = submissions.len(),
        "fetched open and non-revoked submissions",
    );

    let mut issues_created = 0;
    let mut issues_closed = 0;

    for submission in &submissions {
        if submission.ticket_url.is_none() {
            let title = format!(
                "IGNORE IT [import][{}] {}",
                submission.origin, submission.name
            );
            let body = json!({
                "origin": submission.origin,
                "name": submission.name
            });
            let issue =
                service::gitea::create_issue(title, serde_json::to_string_pretty(&body)?, pool)
                    .await?;
            db::place_submission::queries::set_ticket_url(submission.id, issue.url, pool).await?;
            issues_created += 1;
        } else {
            let issue =
                service::gitea::get_issue(submission.ticket_url.clone().unwrap(), pool).await?;

            if issue.state == "closed" {
                db::place_submission::queries::set_closed_at(
                    submission.id,
                    Some(OffsetDateTime::now_utc()),
                    pool,
                )
                .await?;
                issues_closed += 1;
            }
        }
    }

    Ok(Res {
        issues_pending: submissions.len() as i64 - issues_closed,
        issues_created,
        issues_closed,
    })
}
