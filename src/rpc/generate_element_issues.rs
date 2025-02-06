use crate::{
    admin::Admin,
    conf::Conf,
    discord,
    element::{self, Element},
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

pub async fn run(admin: &Admin, pool: &Pool, conf: &Conf) -> Result<Res> {
    let elements = Element::select_all_except_deleted_async(&pool).await?;
    let res = element::service::generate_issues_async(elements, &pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} generated element issues. Affected elements: {}",
            admin.name, res.affected_elements
        ),
    )
    .await;
    Ok(Res {
        started_at: res.started_at,
        finished_at: res.finished_at,
        time_s: res.time_s,
        affected_elements: res.affected_elements,
    })
}
