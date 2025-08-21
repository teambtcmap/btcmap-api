use crate::{
    db::{self, conf::schema::Conf, user::schema::User},
    service::discord,
    Result,
};
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Res {
    pub elements_affected: i64,
    pub time_sec: f64,
}

pub async fn run(requesting_user: &User, pool: &Pool, conf: &Conf) -> Result<Res> {
    let started_at = OffsetDateTime::now_utc();
    let elements =
        db::element::queries::select_updated_since(OffsetDateTime::UNIX_EPOCH, None, true, pool)
            .await?;
    let mut elements_affected = 0;
    for element in elements {
        let comments =
            db::element_comment::queries::select_by_element_id(element.id, false, i64::MAX, pool)
                .await?;
        let new_len = comments.len();
        let old_len = element.tag("comments");
        if old_len.is_null() {
            if new_len == 0 {
                // do nothing
            } else {
                db::element::queries::set_tag(element.id, "comments", &new_len.into(), pool)
                    .await?;
                elements_affected += 1;
            }
        } else {
            let old_len = old_len.as_i64().unwrap_or(0) as usize;
            if new_len != old_len {
                db::element::queries::set_tag(element.id, "comments", &new_len.into(), pool)
                    .await?;
                elements_affected += 1;
            }
        }
    }
    discord::send(
        format!(
            "{} generated comment counts for all elements",
            requesting_user.name,
        ),
        discord::Channel::Api,
        conf,
    );
    Ok(Res {
        elements_affected,
        time_sec: (OffsetDateTime::now_utc() - started_at).as_seconds_f64(),
    })
}
