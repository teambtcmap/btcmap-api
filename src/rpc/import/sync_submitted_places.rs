use crate::{
    db::{self},
    service, Result,
};
use deadpool_sqlite::Pool;
use serde::Serialize;

#[derive(Serialize)]
pub struct Res {
    issues_created: i64,
    issues_closed: i64,
}

pub async fn run(pool: &Pool) -> Result<Res> {
    let submissions = db::place_submission::queries::select_open_and_not_revoked(pool).await?;

    for submission in submissions {
        if submission.ticket_url.is_none() {
            service::gitea::create_issue("title".to_string(), "body".to_string(), pool).await?;
        } else {
            service::gitea::get_issue(submission.ticket_url.unwrap(), pool).await?;
        }
    }

    Ok(Res {
        issues_created: 0,
        issues_closed: 0,
    })
}
