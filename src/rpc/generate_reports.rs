use crate::{auth::Token, command, discord, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::info;

#[derive(Deserialize)]
pub struct Args {
    pub token: String,
}

#[derive(Serialize)]
pub struct Res {
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub finished_at: OffsetDateTime,
    pub time_s: f64,
    pub new_reports: i64,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let started_at = OffsetDateTime::now_utc();
    let res = pool
        .get()
        .await?
        .interact(move |conn| command::generate_reports::run(conn))
        .await??;
    if res > 0 {
        let log_message = format!("{} generated {} daily reports", token.owner, res,);
        info!(log_message);
        discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    }
    Ok(Res {
        started_at: OffsetDateTime::now_utc(),
        finished_at: OffsetDateTime::now_utc(),
        time_s: (OffsetDateTime::now_utc() - started_at).as_seconds_f64(),
        new_reports: res as i64,
    })
}
