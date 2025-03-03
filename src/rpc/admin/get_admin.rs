use crate::{admin::Admin, Result};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub id: i64,
}

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub name: String,
    pub allowed_actions: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let res = Admin::select_by_id_async(params.id, pool).await?;
    Ok(Res {
        id: res.id,
        name: res.name,
        allowed_actions: res.allowed_actions,
        created_at: res.created_at,
        updated_at: res.updated_at,
        deleted_at: res.deleted_at,
    })
}
