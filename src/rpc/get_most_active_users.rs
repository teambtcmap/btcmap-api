use crate::{user::OsmUser, Result};
use deadpool_sqlite::Pool;
use regex::Regex;
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
    pub image_url: Option<String>,
    pub tip_address: Option<String>,
    pub edits: i64,
    pub created: i64,
    pub updated: i64,
    pub deleted: i64,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let period_start =
        OffsetDateTime::parse(&format!("{}T00:00:00Z", params.period_start), &Rfc3339)?;
    let period_end = OffsetDateTime::parse(&format!("{}T00:00:00Z", params.period_end), &Rfc3339)?;
    let res =
        OsmUser::select_most_active_async(period_start, period_end, params.limit, pool).await?;
    let res: Vec<ResUser> = res
        .into_iter()
        .map(|it| {
            let re = Regex::new(r"(lightning:[^)]*)").unwrap();
            let tip_address: Vec<_> = re
                .find_iter(&it.description)
                .take(1)
                .map(|m| m.as_str())
                .collect();
            let tip_address = tip_address.first().map(|it| it.to_string());
            ResUser {
                id: it.id,
                name: it.name,
                image_url: it.image_url,
                tip_address,
                edits: it.edits,
                created: it.created,
                updated: it.updated,
                deleted: it.deleted,
            }
        })
        .collect();
    Ok(Res { users: res })
}
