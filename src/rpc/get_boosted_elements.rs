use crate::{
    db::{self},
    Result,
};
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Res {
    pub id: i64,
    pub admin_id: i64,
    pub element_id: i64,
    pub duration_days: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

pub async fn run(pool: &Pool) -> Result<Vec<Res>> {
    let boosts = db::boost::queries::select_all(pool).await?;
    Ok(boosts
        .iter()
        .map(|it| Res {
            id: it.id,
            admin_id: it.admin_id,
            element_id: it.element_id,
            duration_days: it.duration_days,
            created_at: it.created_at,
            updated_at: it.updated_at,
            deleted_at: it.deleted_at,
        })
        .collect())
}
