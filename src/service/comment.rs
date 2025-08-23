use crate::{
    db::{self, element::schema::Element},
    Result,
};
use deadpool_sqlite::Pool;

pub struct RefreshCommentCountTagRes {
    pub previous_count: i64,
    pub current_count: i64,
    pub count_changed: bool,
}

pub async fn refresh_comment_count_tag(
    element: &Element,
    pool: &Pool,
) -> Result<RefreshCommentCountTagRes> {
    let comments =
        db::element_comment::queries::select_by_element_id(element.id, false, i64::MAX, pool)
            .await?;
    let current_count = comments.len() as i64;
    let previous_count = element
        .tags
        .get("comments")
        .map(|it| it.as_i64().unwrap())
        .unwrap_or(0);

    // avoid db writes for performance reasons
    if current_count == previous_count {
        return Ok(RefreshCommentCountTagRes {
            previous_count,
            current_count,
            count_changed: false,
        });
    }

    if current_count > 0 {
        db::element::queries::set_tag(element.id, "comments", &current_count.into(), pool).await?;
    } else {
        // no need to store zero counts
        // but also avoid useless writes
        if element.tags.contains_key("comments") {
            db::element::queries::remove_tag(element.id, "comments", pool).await?;
        }
    }

    Ok(RefreshCommentCountTagRes {
        previous_count,
        current_count,
        count_changed: previous_count != current_count,
    })
}

#[cfg(test)]
mod test {
    use crate::{
        db::{self, test::pool},
        service::overpass::OverpassElement,
        Result,
    };
    use actix_web::test;
    use serde_json::Value;
    use time::OffsetDateTime;

    #[test]
    async fn refresh_comment_count_tag() -> Result<()> {
        let pool = pool();
        let element = db::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let count_res = super::refresh_comment_count_tag(&element, &pool).await?;
        assert_eq!(0, count_res.previous_count);
        assert_eq!(0, count_res.current_count);
        assert_eq!(false, count_res.count_changed);
        let comment = db::element_comment::queries::insert(element.id, "test", &pool).await?;
        let count_res = super::refresh_comment_count_tag(&element, &pool).await?;
        assert_eq!(0, count_res.previous_count);
        assert_eq!(1, count_res.current_count);
        assert_eq!(true, count_res.count_changed);
        let element = db::element::queries::select_by_id(element.id, &pool).await?;
        assert_eq!(Value::Number(1.into()), element.tags["comments"]);
        db::element_comment::queries::set_deleted_at(
            comment.id,
            Some(OffsetDateTime::now_utc()),
            &pool,
        )
        .await?;
        let count_res = super::refresh_comment_count_tag(&element, &pool).await?;
        assert_eq!(1, count_res.previous_count);
        assert_eq!(0, count_res.current_count);
        assert_eq!(true, count_res.count_changed);
        let element = db::element::queries::select_by_id(element.id, &pool).await?;
        assert_eq!(None, element.tags.get("comments"));
        Ok(())
    }
}
