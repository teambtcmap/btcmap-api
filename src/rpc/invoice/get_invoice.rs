use crate::{
    db::{
        self,
        invoice::schema::{Invoice, InvoiceStatus},
    },
    Result,
};
use deadpool_sqlite::Pool;
use matrix_sdk::Client;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub uuid: String,
}

#[derive(Serialize)]
pub struct Res {
    uuid: String,
    status: String,
}

impl From<Invoice> for Res {
    fn from(val: Invoice) -> Self {
        Res {
            uuid: val.uuid,
            status: val.status.into(),
        }
    }
}

pub async fn run(params: Params, pool: &Pool, matrix_client: Option<Client>) -> Result<Res> {
    let mut invoice = db::invoice::queries::select_by_uuid(params.uuid.clone(), pool).await?;
    if invoice.status == InvoiceStatus::Unpaid
        && crate::service::invoice::sync_unpaid_invoice(&invoice, pool, &matrix_client).await?
    {
        invoice = db::invoice::queries::select_by_uuid(params.uuid, pool).await?;
    }
    Ok(invoice.into())
}
