use crate::{invoice::model::Invoice, Result};
use deadpool_sqlite::Pool;
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
            status: val.status,
        }
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let mut invoice = Invoice::select_by_uuid_async(params.uuid.clone(), pool).await?;
    if invoice.status == "unpaid"
        && crate::invoice::service::sync_unpaid_invoice(&invoice, &pool).await?
    {
        invoice = Invoice::select_by_uuid_async(params.uuid, pool).await?;
    }
    Ok(invoice.into())
}
