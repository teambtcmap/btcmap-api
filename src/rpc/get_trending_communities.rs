use crate::{
    admin,
    area::{self, service::TrendingArea},
    Result,
};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use serde::Deserialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const NAME: &str = "get_trending_communities";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub period_start: String,
    pub period_end: String,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Pool>) -> Result<Vec<TrendingArea>> {
    admin::service::check_rpc(&args.password, NAME, &pool).await?;
    let period_start =
        OffsetDateTime::parse(&format!("{}T00:00:00Z", args.period_start), &Rfc3339)?;
    let period_end = OffsetDateTime::parse(&format!("{}T00:00:00Z", args.period_end), &Rfc3339)?;
    pool.get()
        .await?
        .interact(move |conn| {
            area::service::get_trending_areas("community", &period_start, &period_end, conn)
        })
        .await?
}
