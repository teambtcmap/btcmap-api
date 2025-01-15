use super::model::Invoice;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde_json::json;
use std::env;

#[derive(Deserialize)]
pub struct CreateLNbitsInvoiceResponse {
    pub payment_hash: String,
    pub payment_request: String,
}

pub async fn create(description: String, amount_sats: i64, pool: &Pool) -> Result<Invoice> {
    let client = reqwest::Client::new();
    let api_key = env::var("LNBITS_API_KEY").unwrap();
    let args = json!({"out": false, "amount": amount_sats, "memo": description});
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
        description,
        amount_sats,
        lnbits_response.payment_hash,
        lnbits_response.payment_request,
        "unpaid",
        pool,
    )
    .await
}

#[derive(Deserialize)]
pub struct CheckInvoiceResponse {
    pub paid: bool,
}

pub async fn sync_unpaid_invoices(pool: &Pool) -> Result<Vec<Invoice>> {
    let unpaid_invoices = Invoice::select_by_status_async("unpaid", pool).await?;
    let mut affected_invoices = vec![];
    let client = reqwest::Client::new();
    let api_key = env::var("LNBITS_API_KEY").unwrap();
    for invoice in unpaid_invoices {
        let url = format!(
            "https://core.btcmap.org/api/v1/payments/{}",
            invoice.payment_hash,
        );
        let lnbits_response = client.get(url).header("X-Api-Key", &api_key).send().await?;
        if !lnbits_response.status().is_success() {
            return Err("Failed to check LNBITS invoice".into());
        }
        let lnbits_response: CheckInvoiceResponse = lnbits_response.json().await?;
        if lnbits_response.paid {
            Invoice::set_status_async(invoice.id, "paid", pool).await?;
            affected_invoices.push(invoice);
        }
    }
    Ok(affected_invoices)
}
