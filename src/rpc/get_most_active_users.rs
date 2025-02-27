use crate::{user::User, Result};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Deserialize)]
pub struct Params {
    pub period_start: String,
    pub period_end: String,
    pub limit: i64,
}

#[derive(Serialize)]
pub struct Res {
    users: Vec<ResUser>,
}

#[derive(Serialize)]
pub struct ResUser {
    pub id: i64,
    pub name: String,
    pub edits: i64,
    pub created: i64,
    pub updated: i64,
    pub deleted: i64,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let period_start =
        OffsetDateTime::parse(&format!("{}T00:00:00Z", params.period_start), &Rfc3339)?;
    let period_end = OffsetDateTime::parse(&format!("{}T00:00:00Z", params.period_end), &Rfc3339)?;
    let res = User::select_most_active_async(period_start, period_end, params.limit, pool).await?;
    let res: Vec<ResUser> = res
        .into_iter()
        .map(|it| ResUser {
            id: it.id,
            name: it.name,
            edits: it.edits,
            created: it.created,
            updated: it.updated,
            deleted: it.deleted,
        })
        .collect();
    Ok(Res { users: res })
}
