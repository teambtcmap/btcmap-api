use crate::{invoice::model::Invoice, Result};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
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

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    Invoice::select_by_id_async(params.id, &pool)
        .await
        .map(Into::into)
}
