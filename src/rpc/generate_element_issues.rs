use crate::{
    conf::Conf,
    db::{self, user::schema::User},
    element_issue::model::ElementIssue,
    service::{self, discord},
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

pub async fn run(requesting_user: &User, pool: &Pool, conf: &Conf) -> Result<Res> {
    let elements = db::element::queries_async::select_updated_since(
        OffsetDateTime::UNIX_EPOCH,
        None,
        true,
        pool,
    )
    .await?;
    for element in elements {
        if element.deleted_at.is_some() {
            let issues = ElementIssue::select_by_element_id_async(element.id, pool).await?;
            for issue in issues {
                ElementIssue::set_deleted_at_async(issue.id, Some(OffsetDateTime::now_utc()), pool)
                    .await?;
            }
        }
    }
    let elements = db::element::queries_async::select_updated_since(
        OffsetDateTime::UNIX_EPOCH,
        None,
        false,
        pool,
    )
    .await?;
    let res = service::element::generate_issues_async(elements, pool).await?;
    discord::send(
        format!(
            "{} generated element issues. Affected elements: {}",
            requesting_user.name, res.affected_elements
        ),
        discord::Channel::Api,
        conf,
    );
    Ok(Res {
        started_at: res.started_at,
        finished_at: res.finished_at,
        time_s: res.time_s,
        affected_elements: res.affected_elements,
    })
}
