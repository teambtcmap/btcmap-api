use crate::admin;
use crate::boost::Boost;
use crate::Result;
use deadpool_sqlite::Pool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub password: String,
}

pub async fn run_internal(params: Params, pool: &Pool) -> Result<Vec<Boost>> {
    admin::service::check_rpc(params.password, "get_boosted_elements", &pool).await?;
    let boosts = pool
        .get()
        .await?
        .interact(move |conn| Boost::select_all(conn))
        .await??;
    Ok(boosts)
}
