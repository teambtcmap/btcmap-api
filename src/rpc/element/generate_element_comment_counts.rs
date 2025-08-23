use crate::{
    db::{self},
    service::{self},
    Result,
};
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::OffsetDateTime;
use tracing::info;

#[derive(Serialize)]
pub struct Res {
    pub elements_affected: i64,
    pub time_sec: f64,
}

pub async fn run(pool: &Pool) -> Result<Res> {
    let started_at = OffsetDateTime::now_utc();
    let elements =
        db::element::queries::select_updated_since(OffsetDateTime::UNIX_EPOCH, None, true, pool)
            .await?;
    let mut elements_affected = 0;
    for element in elements {
        let refresh_tag_res = service::comment::refresh_comment_count_tag(&element, pool).await?;
        if refresh_tag_res.count_changed {
            info!(
                element.id,
                element.name = element.name(),
                refresh_tag_res.previous_count,
                refresh_tag_res.current_count,
                "updated comment count"
            );
            elements_affected += 1;
        }
    }
    Ok(Res {
        elements_affected,
        time_sec: (OffsetDateTime::now_utc() - started_at).as_seconds_f64(),
    })
}
