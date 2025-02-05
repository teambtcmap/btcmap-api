use crate::boost::Boost;
use crate::Result;
use deadpool_sqlite::Pool;

pub async fn run(pool: &Pool) -> Result<Vec<Boost>> {
    let boosts = pool
        .get()
        .await?
        .interact(move |conn| Boost::select_all(conn))
        .await??;
    Ok(boosts)
}
