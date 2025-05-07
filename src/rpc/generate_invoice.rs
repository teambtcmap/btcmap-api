use crate::{
    conf::Conf,
    db::admin::queries::Admin,
    discord,
    invoice::{self, model::Invoice},
    Result,
};
use deadpool_sqlite::Pool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub amount_sats: i64,
    pub description: String,
}

pub async fn run(params: Params, admin: &Admin, pool: &Pool, conf: &Conf) -> Result<Invoice> {
    let invoice = invoice::service::create(params.description, params.amount_sats, pool).await?;
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
