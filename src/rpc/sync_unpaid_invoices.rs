use crate::{
    admin, discord,
    invoice::{self, model::Invoice},
    Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

const NAME: &str = "sync_unpaid_invoices";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Vec<Invoice>> {
    let admin = admin::service::check_rpc(args.password, NAME, &pool).await?;
    let affected_invoices = invoice::service::sync_unpaid_invoices(&pool).await?;
    if affected_invoices.len() > 0 {
        let log_message = format!(
            "{} synced unpaid invoices, marking {} invoices as paid",
            admin.name,
            affected_invoices.len(),
        );
        info!(log_message);
        discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    }
    Ok(affected_invoices)
}
