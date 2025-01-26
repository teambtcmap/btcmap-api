use crate::{
    admin,
    conf::Conf,
    discord,
    invoice::{self, model::Invoice},
    Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

const NAME: &str = "generate_invoice";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub amount_sats: i64,
    pub description: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Invoice> {
    let admin = admin::service::check_rpc(args.password, NAME, &pool).await?;
    let invoice = invoice::service::create(args.description, args.amount_sats, &pool).await?;
    let log_message = format!(
        "Admin {} generated invoice {} for {} sats with the following description: {}",
        admin.name, invoice.id, args.amount_sats, invoice.description,
    );
    info!(log_message);
    let conf = Conf::select_async(&pool).await?;
    discord::post_message(conf.discord_webhook_api, log_message).await;
    Ok(invoice)
}
