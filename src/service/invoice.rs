use crate::{
    db::{
        self,
        invoice::schema::{Invoice, InvoiceStatus},
    },
    service::{
        self,
        matrix::{self, ROOM_PLACE_BOOSTS, ROOM_PLACE_COMMENTS},
    },
    Result,
};
use base64::Engine;
use deadpool_sqlite::Pool;
use matrix_sdk::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

#[derive(Deserialize)]
pub struct CreateLNbitsInvoiceResponse {
    pub payment_hash: String,
    pub bolt11: String,
}

#[derive(Deserialize)]
pub struct CreateLndInvoiceResponse {
    pub r_hash: String,
    pub payment_request: String,
}

pub async fn create(
    source: &str,
    description: String,
    amount_sats: i64,
    pool: &Pool,
) -> Result<Invoice> {
    let conf = db::conf::queries::select(pool).await?;
    let client = reqwest::Client::new();

    if source == "lnbits" {
        if conf.lnbits_invoice_key.is_empty() {
            Err("lnbits invoice key is not set")?
        }
        let args = json!({"out": false, "amount": amount_sats, "memo": description});
        let lnbits_response = client
            .post("https://core.btcmap.org/api/v1/payments")
            .header("X-Api-Key", &conf.lnbits_invoice_key)
            .json(&args)
            .send()
            .await?;
        if !lnbits_response.status().is_success() {
            return Err("Failed to generate LNBITS invoice".into());
        }
        let lnbits_response: CreateLNbitsInvoiceResponse = lnbits_response.json().await?;
        let invoice = db::invoice::queries::insert(
            source,
            description,
            amount_sats,
            lnbits_response.payment_hash,
            lnbits_response.bolt11,
            InvoiceStatus::Unpaid,
            pool,
        )
        .await?;
        Ok(invoice)
    } else if source == "lnd" {
        if conf.lnd_invoices_macaroon.is_empty() {
            Err("lnd invoices macaroon is not set")?
        }
        let args = json!({"value": amount_sats, "memo": description, "expiry": 3600});
        let lnd_response = client
            .post("https://lnd.btcmap.org/v1/invoices")
            .header("Grpc-Metadata-macaroon", &conf.lnd_invoices_macaroon)
            .json(&args)
            .send()
            .await?;
        if !lnd_response.status().is_success() {
            return Err("Failed to generate lnd invoice".into());
        }
        let lnd_response: CreateLndInvoiceResponse = lnd_response.json().await?;
        let payment_hash_bytes = base64::engine::general_purpose::STANDARD
            .decode(lnd_response.r_hash)
            .unwrap();
        let payment_hash_hex: String = payment_hash_bytes
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect();
        let invoice = db::invoice::queries::insert(
            source,
            description,
            amount_sats,
            payment_hash_hex,
            lnd_response.payment_request,
            InvoiceStatus::Unpaid,
            pool,
        )
        .await?;
        Ok(invoice)
    } else {
        Err("unknown source")?
    }
}

#[derive(Deserialize)]
pub struct CheckInvoiceResponse {
    pub paid: bool,
}

#[derive(Deserialize)]
pub struct CheckLndInvoiceResponse {
    pub state: String,
}

pub async fn sync_unpaid_invoices(pool: &Pool, matrix_client: &Option<Client>) -> Result<i64> {
    let conf = db::conf::queries::select(pool).await?;
    if conf.lnbits_invoice_key.is_empty() {
        Err("lnbits invoice key is not set")?
    }
    let unpaid_invoices =
        db::invoice::queries::select_by_status(InvoiceStatus::Unpaid, pool).await?;
    let now = OffsetDateTime::now_utc();
    let hour_ago = now.saturating_sub(Duration::hours(1)).format(&Rfc3339)?;
    let unpaid_invoices: Vec<Invoice> = unpaid_invoices
        .into_iter()
        .filter(|it| it.created_at > hour_ago)
        .collect();
    let mut affected_invoices = vec![];
    for invoice in unpaid_invoices {
        if sync_unpaid_invoice(&invoice, pool, matrix_client).await? {
            affected_invoices.push(invoice);
        }
    }
    Ok(affected_invoices.len() as i64)
}

// Returns true if invoice was unpaid and became paid
pub async fn sync_unpaid_invoice(
    invoice: &Invoice,
    pool: &Pool,
    matrix_client: &Option<Client>,
) -> Result<bool> {
    if invoice.status != InvoiceStatus::Unpaid {
        return Ok(false);
    }
    let conf = db::conf::queries::select(pool).await?;
    let client = reqwest::Client::new();
    if invoice.source == "lnbits" {
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
            db::invoice::queries::set_status(invoice.id, InvoiceStatus::Paid, pool).await?;
            on_invoice_paid(invoice, pool, matrix_client).await?;
            return Ok(true);
        }
    } else if invoice.source == "lnd" {
        let url = format!("https://lnd.btcmap.org/v1/invoice/{}", invoice.payment_hash,);
        let lnd_response = client
            .get(url)
            .header("Grpc-Metadata-macaroon", &conf.lnd_invoices_macaroon)
            .send()
            .await?;
        if !lnd_response.status().is_success() {
            return Err("Failed to check lnd invoice".into());
        }
        let lnd_response: CheckLndInvoiceResponse = lnd_response.json().await?;
        if lnd_response.state == "SETTLED" {
            db::invoice::queries::set_status(invoice.id, InvoiceStatus::Paid, pool).await?;
            on_invoice_paid(invoice, pool, matrix_client).await?;
            return Ok(true);
        }
    } else {
        return Err("unknown invoice source".into());
    }
    Ok(false)
}

pub async fn on_invoice_paid(
    invoice: &Invoice,
    pool: &Pool,
    matrix_client: &Option<Client>,
) -> Result<()> {
    if invoice.description.starts_with("element_comment") {
        let parts: Vec<&str> = invoice.description.split(":").collect();
        let id = parts.get(1).unwrap_or(&"");
        let action = parts.get(2).unwrap_or(&"");
        if id.is_empty() || action.is_empty() {
            return Ok(());
        }
        let id = id.parse::<i64>().unwrap_or(0);
        if *action == "publish" {
            let comment = db::element_comment::queries::select_by_id(id, pool).await?;
            let element = db::element::queries::select_by_id(comment.element_id, pool).await?;
            db::element_comment::queries::set_deleted_at(id, None, pool).await?;
            service::comment::refresh_comment_count_tag(&element, pool).await?;
            let message = format!(
                "{} https://btcmap.org/merchant/{}",
                comment.comment, element.id,
            );
            service::matrix::send_message(matrix_client, ROOM_PLACE_COMMENTS, &message);
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
        let Ok(element) =
            db::element::queries::select_by_id_or_osm_id(element_id.to_string(), pool).await
        else {
            return Ok(());
        };
        let boost_expires = if element.tags.contains_key("boost:expires") {
            let now = OffsetDateTime::now_utc();
            let now_str = now.format(&Rfc3339)?;

            let current_boost_expires = element.tags["boost:expires"].as_str().unwrap_or(&now_str);
            let mut current_boost_expires =
                OffsetDateTime::parse(current_boost_expires, &Rfc3339).unwrap_or(now);

            if current_boost_expires < now {
                current_boost_expires = now
            }

            current_boost_expires + Duration::days(days)
        } else {
            OffsetDateTime::now_utc().saturating_add(Duration::days(days))
        };
        db::element::queries::set_tag(
            element_id,
            "boost:expires",
            &Value::String(boost_expires.format(&Rfc3339)?),
            pool,
        )
        .await?;
        let message = format!(
            "Boosted element since invoice has been paid (id = {}, name = {}, days = {})",
            element_id,
            element.name(),
            days,
        );
        matrix::send_message(matrix_client, ROOM_PLACE_BOOSTS, &message);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::{
        db::{self, test::pool},
        service::overpass::OverpassElement,
        Result,
    };
    use actix_web::test;
    use serde_json::Value;
    use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

    #[test]
    async fn on_invoice_paid_on_unboosted_element() -> Result<()> {
        let pool = pool();
        db::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let invoice = db::invoice::queries::insert(
            "src",
            "element_boost:1:10",
            0,
            "",
            "",
            db::invoice::schema::InvoiceStatus::Unpaid,
            &pool,
        )
        .await?;
        super::on_invoice_paid(&invoice, &pool, &None).await?;
        let element = db::element::queries::select_by_id(1, &pool).await?;
        assert!(element.tags.contains_key("boost:expires"));
        let boost_expires =
            OffsetDateTime::parse(element.tags["boost:expires"].as_str().unwrap(), &Rfc3339)?;
        assert_eq!(9, (boost_expires - OffsetDateTime::now_utc()).whole_days());
        Ok(())
    }

    #[test]
    async fn on_invoice_paid_on_boosted_element() -> Result<()> {
        let pool = pool();
        let element = db::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let old_boost_expires = OffsetDateTime::now_utc().saturating_sub(Duration::days(5));
        let old_boost_expires = old_boost_expires.format(&Rfc3339)?;
        db::element::queries::set_tag(
            element.id,
            "boost:expires",
            &Value::String(old_boost_expires),
            &pool,
        )
        .await?;
        let invoice = db::invoice::queries::insert(
            "src",
            "element_boost:1:10",
            0,
            "",
            "",
            db::invoice::schema::InvoiceStatus::Unpaid,
            &pool,
        )
        .await?;
        super::on_invoice_paid(&invoice, &pool, &None).await?;
        let element = db::element::queries::select_by_id(1, &pool).await?;
        assert!(element.tags.contains_key("boost:expires"));
        let boost_expires =
            OffsetDateTime::parse(element.tags["boost:expires"].as_str().unwrap(), &Rfc3339)?;
        assert_eq!(9, (boost_expires - OffsetDateTime::now_utc()).whole_days());
        Ok(())
    }
}
