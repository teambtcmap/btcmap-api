use crate::{
    admin,
    conf::Conf,
    discord,
    element::{self, Element},
    Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::Data;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;

pub const NAME: &str = "generate_element_issues";

#[derive(Deserialize)]
pub struct Params {
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

pub async fn run(
    jsonrpc_v2::Params(params): jsonrpc_v2::Params<Params>,
    pool: Data<Arc<Pool>>,
    conf: Data<Arc<Conf>>,
) -> Result<Res> {
    run_internal(params, &pool, &conf).await
}

pub async fn run_internal(params: Params, pool: &Pool, conf: &Conf) -> Result<Res> {
    let admin = admin::service::check_rpc(params.password, NAME, &pool).await?;
    let elements = Element::select_all_except_deleted_async(&pool).await?;
    let res = element::service::generate_issues_async(elements, &pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} generated element issues, affecting {} elements",
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
