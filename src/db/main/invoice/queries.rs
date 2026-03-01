use super::{
    blocking_queries,
    schema::{Invoice, InvoiceStatus},
};
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn insert(
    source: impl Into<String>,
    description: impl Into<String>,
    amount_sats: i64,
    payment_hash: impl Into<String>,
    payment_request: impl Into<String>,
    status: InvoiceStatus,
    pool: &Pool,
) -> Result<Invoice> {
    let source = source.into();
    let description = description.into();
    let payment_hash = payment_hash.into();
    let payment_request = payment_request.into();
    pool.get()
        .await?
        .interact(move |conn| {
            blocking_queries::insert(
                &source,
                description,
                amount_sats,
                payment_hash,
                payment_request,
                status,
                conn,
            )
        })
        .await?
}

pub async fn select_by_status(status: InvoiceStatus, pool: &Pool) -> Result<Vec<Invoice>> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_status(status, conn))
        .await?
}

pub async fn select_by_uuid(uuid: impl Into<String>, pool: &Pool) -> Result<Invoice> {
    let uuid = uuid.into();
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::select_by_uuid(&uuid, conn))
        .await?
}

pub async fn set_status(invoice_id: i64, status: InvoiceStatus, pool: &Pool) -> Result<Invoice> {
    pool.get()
        .await?
        .interact(move |conn| blocking_queries::set_status(invoice_id, status, conn))
        .await?
}
