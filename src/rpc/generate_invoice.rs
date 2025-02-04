use crate::{
    admin,
    conf::Conf,
    discord,
    invoice::{self, model::Invoice},
    Result,
};
use deadpool_sqlite::Pool;
use serde::Deserialize;

pub const NAME: &str = "generate_invoice";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
    pub amount_sats: i64,
    pub description: String,
}

pub async fn run_internal(params: Params, pool: &Pool, conf: &Conf) -> Result<Invoice> {
    let admin = admin::service::check_rpc(params.password, NAME, &pool).await?;
    let invoice = invoice::service::create(params.description, params.amount_sats, &pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} generated invoice {} for {} sats with the following description: {}",
            admin.name, invoice.id, params.amount_sats, invoice.description,
        ),
    )
    .await;
    Ok(invoice)
}
