use crate::{
    conf::Conf, db::admin::queries::Admin, discord, element::Element,
    element_comment::ElementComment, Result,
};
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Res {
    pub elements_affected: i64,
    pub time_sec: f64,
}

pub async fn run(admin: &Admin, pool: &Pool, conf: &Conf) -> Result<Res> {
    let started_at = OffsetDateTime::now_utc();
    let elements = Element::select_all_async(None, pool).await?;
    let mut elements_affected = 0;
    for element in elements {
        let comments =
            ElementComment::select_by_element_id_async(element.id, false, i64::MAX, pool).await?;
        let new_len = comments.len();
        let old_len = element.tag("comments");
        if old_len.is_null() {
            if new_len == 0 {
                // do nothing
            } else {
                Element::set_tag_async(element.id, "comments", &new_len.into(), pool).await?;
                elements_affected += 1;
            }
        } else {
            let old_len = old_len.as_i64().unwrap_or(0) as usize;
            if new_len != old_len {
                Element::set_tag_async(element.id, "comments", &new_len.into(), pool).await?;
                elements_affected += 1;
            }
        }
    }
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} generated comment counts for all elements",
            admin.name,
        ),
    )
    .await;
    Ok(Res {
        elements_affected,
        time_sec: (OffsetDateTime::now_utc() - started_at).as_seconds_f64(),
    })
}
