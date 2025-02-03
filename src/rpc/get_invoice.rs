use crate::{admin, invoice::model::Invoice, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::Data;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const NAME: &str = "get_invoice";

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
    pub id: i64,
}

#[derive(Serialize)]
pub struct Res {
    id: i64,
    description: String,
    status: String,
}

impl From<Invoice> for Res {
    fn from(val: Invoice) -> Self {
        Res {
            id: val.id,
            description: val.description,
            status: val.status,
        }
    }
}

pub async fn run(
    jsonrpc_v2::Params(params): jsonrpc_v2::Params<Params>,
    pool: Data<Arc<Pool>>,
) -> Result<Res> {
    run_internal(params, &pool).await
}

pub async fn run_internal(params: Params, pool: &Pool) -> Result<Res> {
    admin::service::check_rpc(params.password, NAME, &pool).await?;
    Invoice::select_by_id_async(params.id, &pool)
        .await
        .map(Into::into)
}
