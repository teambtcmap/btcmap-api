use crate::{db::admin::queries::Admin, Result};
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

impl From<Admin> for Res {
    fn from(val: Admin) -> Self {
        Self {
            id: val.id,
            name: val.name,
            allowed_actions: val.roles,
            created_at: val.created_at,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    crate::db::admin::queries_async::select_by_id(params.id, pool)
        .await
        .map(Into::into)
}
