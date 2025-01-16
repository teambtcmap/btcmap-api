use super::model::Invoice;
use crate::{discord, element_comment::ElementComment, Result};
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde_json::json;
use std::env;
use tracing::info;

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
    let invoice = Invoice::insert_async(
        description,
        amount_sats,
        lnbits_response.payment_hash,
        lnbits_response.payment_request,
        "unpaid",
        pool,
    )
    .await?;
    discord::send_message_to_channel(
        &format!(
            "created invoice id = {} sats = {} description = {}",
            invoice.id, invoice.amount_sats, invoice.description,
        ),
        discord::CHANNEL_API,
    )
    .await;
    Ok(invoice)
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
            on_invoice_paid(&invoice, pool).await?;
            affected_invoices.push(invoice);
        }
    }
    Ok(affected_invoices)
}

pub async fn on_invoice_paid(invoice: &Invoice, pool: &Pool) -> Result<()> {
    info!(invoice.id, invoice.description, "invoice has been paid");
    discord::send_message_to_channel(
        &format!(
            "invoice id = {} sats = {} description = {} has been paid",
            invoice.id, invoice.amount_sats, invoice.description,
        ),
        discord::CHANNEL_API,
    )
    .await;
    if invoice.description.starts_with("element_comment") {
        info!("parsing element comment invoice");
        let parts: Vec<&str> = invoice.description.split(":").collect();
        let id = parts.get(1).unwrap_or(&"");
        let action = parts.get(2).unwrap_or(&"");
        info!(id, action, "parsed element_comment id and action");
        if id.is_empty() || action.is_empty() {
            return Ok(());
        }
        let id = id.parse::<i64>().unwrap_or(0);
        if *action == "publish" {
            let comment = ElementComment::select_by_id_async(id, pool).await?;
            if comment.is_some() {
                ElementComment::set_deleted_at_async(id, None, pool).await?;
            }
        }
    }
    Ok(())
}
