use crate::{
    db::{self},
    service, Result,
};
use deadpool_sqlite::Pool;
use serde::Serialize;
use serde_json::json;
use tracing::info;

#[derive(Serialize)]
pub struct Res {
    issues_created: i64,
    issues_closed: i64,
}

pub async fn run(pool: &Pool) -> Result<Res> {
    let submissions = db::place_submission::queries::select_open_and_not_revoked(pool).await?;
    info!(
        len = submissions.len(),
        "fetched open and non-revoked submissions",
    );

    for submission in submissions {
        if submission.ticket_url.is_none() {
            let title = format!("IGNORE IT [import][{}] {}", submission.origin, submission.name);

            let body = json!({
                "origin": submission.origin,
                "name": submission.name
            });

            let issue = service::gitea::create_issue(
                title,
                serde_json::to_string_pretty(&body)?,
                pool,
            )
            .await?;

            
        } else {
            //service::gitea::get_issue(submission.ticket_url.unwrap(), pool).await?;
        }
    }

    Ok(Res {
        issues_created: 0,
        issues_closed: 0,
    })
}
