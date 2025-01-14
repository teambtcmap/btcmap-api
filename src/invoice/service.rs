use std::env;

use super::model::Invoice;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct CreateLNbitsInvoiceResponse {
    pub payment_hash: String,
    pub payment_request: String,
}

pub async fn create(amount_sats: i64, pool: &Pool) -> Result<Invoice> {
    let client = reqwest::Client::new();
    let api_key = env::var("LNBITS_API_KEY").unwrap();
    let args = json!({"out": false, "amount": amount_sats, "memo": "TEST"});
    let lnbits_response = client
        .post("https://core.btcmap.org/api/v1/payments")
        .header("X-Api-Key", api_key)
        .json(&args)
        .send()
        .await?;
    if !lnbits_response.status().is_success() {
        return Err("Failed to generate LNBITS invoice".into());
    }
    let lnbits_response: CreateLNbitsInvoiceResponse = lnbits_response.json().await?;
    Invoice::insert_async(
        amount_sats,
        lnbits_response.payment_hash,
        lnbits_response.payment_request,
        "unpaid",
        pool,
    )
    .await
}
