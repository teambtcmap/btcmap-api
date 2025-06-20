use crate::{
    conf::Conf,
    db::{invoice::schema::Invoice, user::schema::User},
    discord,
    invoice::{self},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub amount_sats: i64,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct Res {
    pub uuid: String,
    pub payment_request: String,
}

impl From<Invoice> for Res {
    fn from(invoice: Invoice) -> Self {
        Res {
            uuid: invoice.uuid,
            payment_request: invoice.payment_request,
        }
    }
}

pub async fn run(params: Params, author: &User, pool: &Pool, conf: &Conf) -> Result<Res> {
    let invoice = invoice::service::create(
        params.description.unwrap_or_default(),
        params.amount_sats,
        pool,
    )
    .await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "{} created a new invoice (uuid: {}, sats: {}, description: {})",
            author.name, invoice.uuid, params.amount_sats, invoice.description,
        ),
    )
    .await;
    Ok(invoice.into())
}
