use crate::{
    admin::Admin,
    discord,
    element::{self, Element},
    Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::info;

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
}

#[derive(Serialize)]
pub struct Res {
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub finished_at: OffsetDateTime,
    pub time_s: f64,
    pub affected_elements: i64,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let admin = pool
        .get()
        .await?
        .interact(move |conn| Admin::select_by_password(&args.password, conn))
        .await??
        .unwrap();
    let elements: Vec<Element> = pool
        .get()
        .await?
        .interact(move |conn| Element::select_all(None, conn))
        .await??;
    let elements: Vec<Element> = elements
        .into_iter()
        .filter(|it| it.deleted_at.is_none())
        .collect();
    let res = pool
        .get()
        .await?
        .interact(move |conn| element::service::generate_issues(elements.iter().collect(), conn))
        .await??;
    let log_message = format!(
        "{} generated element issues, affecting {} elements",
        admin.name, res.affected_elements,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(Res {
        started_at: res.started_at,
        finished_at: res.finished_at,
        time_s: res.time_s,
        affected_elements: res.affected_elements,
    })
}
