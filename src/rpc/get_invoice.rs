use crate::{admin, invoice::model::Invoice, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const NAME: &str = "get_invoice";

#[derive(Deserialize)]
pub struct Args {
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

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Res> {
    admin::service::check_rpc(args.password, NAME, &pool).await?;
    Invoice::select_by_id_async(args.id, &pool)
        .await
        .map(Into::into)
}
