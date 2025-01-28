use super::model::Invoice;
use crate::{conf::Conf, discord, element::Element, element_comment::ElementComment, Result};
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

#[derive(Deserialize)]
pub struct CreateLNbitsInvoiceResponse {
    pub payment_hash: String,
    pub payment_request: String,
}

pub async fn create(description: String, amount_sats: i64, pool: &Pool) -> Result<Invoice> {
    let conf = Conf::select_async(&pool).await?;
    if conf.lnbits_invoice_key.is_empty() {
        Err("lnbits invoice key is not set")?
    }
    let client = reqwest::Client::new();
    let args = json!({"out": false, "amount": amount_sats, "memo": description});
    let lnbits_response = client
        .post("https://core.btcmap.org/api/v1/payments")
        .header("X-Api-Key", conf.lnbits_invoice_key)
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
    discord::post_message(
        conf.discord_webhook_api,
        format!(
            "Created invoice (id = {}, sat = {}, description = {})",
            invoice.id, invoice.amount_sats, invoice.description,
        ),
    )
    .await;
    Ok(invoice)
}

#[derive(Deserialize)]
pub struct CheckInvoiceResponse {
    pub paid: bool,
}

pub async fn sync_unpaid_invoices(pool: &Pool) -> Result<Vec<Invoice>> {
    let conf = Conf::select_async(&pool).await?;
    if conf.lnbits_invoice_key.is_empty() {
        Err("lnbits invoice key is not set")?
    }
    let unpaid_invoices = Invoice::select_by_status_async("unpaid", pool).await?;
    let now = OffsetDateTime::now_utc();
    let unpaid_invoices: Vec<Invoice> = unpaid_invoices
        .into_iter()
        .filter(|it| it.created_at > now.saturating_sub(Duration::hours(1)))
        .collect();
    let mut affected_invoices = vec![];
    let client = reqwest::Client::new();
    for invoice in unpaid_invoices {
        let url = format!(
            "https://core.btcmap.org/api/v1/payments/{}",
            invoice.payment_hash,
        );
        let lnbits_response = client
            .get(url)
            .header("X-Api-Key", &conf.lnbits_invoice_key)
            .send()
            .await?;
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
    let conf = Conf::select_async(&pool).await?;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Invoice has been paid (id = {}, sat = {}, description = {})",
            invoice.id, invoice.amount_sats, invoice.description,
        ),
    )
    .await;
    if invoice.description.starts_with("element_comment") {
        let parts: Vec<&str> = invoice.description.split(":").collect();
        let id = parts.get(1).unwrap_or(&"");
        let action = parts.get(2).unwrap_or(&"");
        if id.is_empty() || action.is_empty() {
            return Ok(());
        }
        let id = id.parse::<i64>().unwrap_or(0);
        if *action == "publish" {
            let comment = ElementComment::select_by_id_async(id, pool).await?;
            if comment.is_some() {
                ElementComment::set_deleted_at_async(id, None, pool).await?;
                discord::post_message(
                    &conf.discord_webhook_api,
                    format!(
                        "Published comment since invoice has been paid: {}",
                        comment.unwrap().comment,
                    ),
                )
                .await;
            }
        }
    }

    if invoice.description.starts_with("element_boost") {
        let parts: Vec<&str> = invoice.description.split(":").collect();
        let element_id = parts.get(1).unwrap_or(&"");
        let element_id = element_id.parse::<i64>().unwrap_or(0);
        let days = parts.get(2).unwrap_or(&"");
        let days = days.parse::<i64>().unwrap_or(0);
        if element_id == 0 || days == 0 {
            return Ok(());
        }
        let element = Element::select_by_id_or_osm_id_async(element_id.to_string(), pool).await?;
        let Some(element) = element else {
            return Ok(());
        };
        let boost_expires = if element.tags.contains_key("boost:expires") {
            let now = OffsetDateTime::now_utc();
            let now_str = now.format(&Rfc3339)?;
            let old_boost_expires = element.tags["boost:expires"].as_str().unwrap_or(&now_str);
            let old_boost_expires =
                OffsetDateTime::parse(old_boost_expires, &Rfc3339).unwrap_or(now);
            old_boost_expires + Duration::days(days)
        } else {
            OffsetDateTime::now_utc().saturating_add(Duration::days(days))
        };
        Element::set_tag_async(
            element_id,
            "boost:expires",
            &Value::String(boost_expires.format(&Rfc3339)?),
            pool,
        )
        .await?;
        discord::post_message(
            conf.discord_webhook_api,
            format!(
                "Boosted element since invoice has been paid (id = {}, name = {}, days = {})",
                element_id,
                element.name(),
                days,
            ),
        )
        .await;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::{
        element::Element, invoice::model::Invoice, osm::overpass::OverpassElement, test::mock_db,
        Result,
    };
    use actix_web::test;
    use serde_json::Value;
    use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

    #[test]
    async fn on_invoice_paid_on_unboosted_element() -> Result<()> {
        let db = mock_db().await;
        Element::insert(&OverpassElement::mock(1), &db.conn)?;
        let invoice = Invoice::insert("element_boost:1:10", 0, "", "", "", &db.conn)?;
        super::on_invoice_paid(&invoice, &db.pool).await?;
        let element = Element::select_by_id(1, &db.conn)?.unwrap();
        assert!(element.tags.contains_key("boost:expires"));
        let boost_expires =
            OffsetDateTime::parse(element.tags["boost:expires"].as_str().unwrap(), &Rfc3339)?;
        assert_eq!(9, (boost_expires - OffsetDateTime::now_utc()).whole_days());
        Ok(())
    }

    #[test]
    async fn on_invoice_paid_on_boosted_element() -> Result<()> {
        let db = mock_db().await;
        let element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        let old_boost_expires = OffsetDateTime::now_utc().saturating_sub(Duration::days(5));
        let old_boost_expires = old_boost_expires.format(&Rfc3339)?;
        Element::set_tag(
            element.id,
            "boost:expires",
            &Value::String(old_boost_expires),
            &db.conn,
        )?;
        let invoice = Invoice::insert("element_boost:1:10", 0, "", "", "", &db.conn)?;
        super::on_invoice_paid(&invoice, &db.pool).await?;
        let element = Element::select_by_id(1, &db.conn)?.unwrap();
        assert!(element.tags.contains_key("boost:expires"));
        let boost_expires =
            OffsetDateTime::parse(element.tags["boost:expires"].as_str().unwrap(), &Rfc3339)?;
        assert_eq!(4, (boost_expires - OffsetDateTime::now_utc()).whole_days());
        Ok(())
    }
}
