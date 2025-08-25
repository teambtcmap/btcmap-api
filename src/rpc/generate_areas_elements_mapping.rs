use crate::{
    db::{self},
    service::{self, area_element::Diff},
    Result,
};
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Res {
    pub affected_elements: Vec<Diff>,
}

pub async fn run(pool: &Pool) -> Result<Res> {
    let elements =
        db::element::queries::select_updated_since(OffsetDateTime::UNIX_EPOCH, None, true, pool)
            .await?;
    let affected_elements = service::area_element::generate_mapping(&elements, pool).await?;
    Ok(Res { affected_elements })
}
