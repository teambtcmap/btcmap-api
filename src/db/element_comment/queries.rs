use super::schema::{self, Columns, ElementComment};
use crate::Result;
use rusqlite::{named_params, params, Connection};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn insert(
    element_id: i64,
    comment: impl Into<String>,
    conn: &Connection,
) -> Result<ElementComment> {
    let sql = format!(
        r#"
            INSERT INTO {table} (
                {element_id},
                {comment}
            ) VALUES (
                ?1,
                ?2
            )
        "#,
        table = schema::TABLE_NAME,
        element_id = Columns::ElementId.as_str(),
        comment = Columns::Comment.as_str(),
    );
    conn.execute(&sql, params![element_id, comment.into()])?;
    select_by_id(conn.last_insert_rowid(), conn)
}

pub fn select_updated_since(
    updated_since: &OffsetDateTime,
    include_deleted: bool,
    limit: Option<i64>,
    conn: &Connection,
) -> Result<Vec<ElementComment>> {
    let include_deleted_sql = if include_deleted {
        ""
    } else {
        "AND deleted_at IS NULL"
    };
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {updated_at} > :updated_since {include_deleted_sql}
            ORDER BY {updated_at}, {id}
            LIMIT :limit
        "#,
        projection = ElementComment::projection(),
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(
            named_params! {
                ":updated_since": updated_since.format(&Rfc3339)?,
                ":limit": limit.unwrap_or(i64::MAX),
            },
            ElementComment::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_latest(limit: i64, conn: &Connection) -> Result<Vec<ElementComment>> {
    let sql: String = format!(
        r#"
            SELECT {projection}
            FROM {table}
            ORDER BY {updated_at} DESC, {id} DESC
            LIMIT ?1
        "#,
        projection = ElementComment::projection(),
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    let res = conn
        .prepare(&sql)?
        .query_map(params![limit], ElementComment::mapper())?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(res)
}

pub fn select_created_between(
    period_start: &OffsetDateTime,
    period_end: &OffsetDateTime,
    conn: &Connection,
) -> Result<Vec<ElementComment>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {created_at} > ?1 AND {created_at} < ?2
            ORDER BY {updated_at}, {id}
        "#,
        projection = ElementComment::projection(),
        table = schema::TABLE_NAME,
        created_at = Columns::CreatedAt.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(
            params![period_start.format(&Rfc3339)?, period_end.format(&Rfc3339)?,],
            ElementComment::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_element_id(
    element_id: i64,
    include_deleted: bool,
    limit: i64,
    conn: &Connection,
) -> Result<Vec<ElementComment>> {
    let include_deleted_sql = if include_deleted {
        ""
    } else {
        "AND deleted_at IS NULL"
    };
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {element_id} = :element_id {include_deleted_sql}
            ORDER BY {updated_at}, {id}
            LIMIT :limit
        "#,
        projection = ElementComment::projection(),
        table = schema::TABLE_NAME,
        element_id = Columns::ElementId.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(
            named_params! {
                ":element_id": element_id,
                ":limit": limit,
            },
            ElementComment::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<ElementComment> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = ElementComment::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_str(),
    );
    Ok(conn.query_row(&sql, params![id], ElementComment::mapper())?)
}

#[cfg(test)]
pub fn set_created_at(
    id: i64,
    created_at: OffsetDateTime,
    conn: &Connection,
) -> Result<ElementComment> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {created_at} = ?2
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        created_at = Columns::CreatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![id, created_at.format(&Rfc3339)?])?;
    select_by_id(id, conn)
}

#[cfg(test)]
pub fn set_updated_at(
    id: i64,
    updated_at: OffsetDateTime,
    conn: &Connection,
) -> Result<ElementComment> {
    let sql = format!(
        r#"
                    UPDATE {table}
                    SET {updated_at} = ?2
                    WHERE {id} = ?1
                "#,
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![id, updated_at.format(&Rfc3339)?])?;
    select_by_id(id, conn)
}

pub fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    conn: &Connection,
) -> Result<ElementComment> {
    match deleted_at {
        Some(deleted_at) => {
            let sql = format!(
                r#"
                    UPDATE {table}
                    SET {deleted_at} = ?2
                    WHERE {id} = ?1
                "#,
                table = schema::TABLE_NAME,
                deleted_at = Columns::DeletedAt.as_str(),
                id = Columns::Id.as_str(),
            );
            conn.execute(&sql, params![id, deleted_at.format(&Rfc3339)?])?;
        }
        None => {
            let sql = format!(
                r#"
                    UPDATE {table}
                    SET {deleted_at} = NULL
                    WHERE {id} = ?1
                "#,
                table = schema::TABLE_NAME,
                deleted_at = Columns::DeletedAt.as_str(),
                id = Columns::Id.as_str(),
            );
            conn.execute(&sql, params![id])?;
        }
    };
    select_by_id(id, conn)
}

#[cfg(test)]
mod test {
    use crate::{db::test::conn, Result};
    use time::{Duration, OffsetDateTime};

    #[test]
    fn insert_and_select_by_id() -> Result<()> {
        let conn = conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;
        let now = OffsetDateTime::now_utc();

        // Test insert
        let inserted = super::insert(1, "Test comment", &conn)?;

        // Verify select_by_id
        let selected = super::select_by_id(inserted.id, &conn)?;
        assert_eq!(inserted.id, selected.id);
        assert_eq!(inserted.element_id, 1);
        assert_eq!(inserted.comment, "Test comment");
        assert!(selected.created_at <= now);
        assert!(selected.updated_at <= now);
        assert_eq!(selected.deleted_at, None);

        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;

        let time1 = OffsetDateTime::now_utc().saturating_add(Duration::hours(-1));
        let comment1 = super::insert(1, "First", &conn)?;
        let _comment1 = super::set_updated_at(comment1.id, time1, &conn)?;

        let time2 = OffsetDateTime::now_utc().saturating_add(Duration::hours(1));
        let comment2 = super::insert(1, "Second", &conn)?;
        let comment2 = super::set_updated_at(comment2.id, time2, &conn)?;

        // Test updated_since
        let results = super::select_updated_since(&OffsetDateTime::now_utc(), false, None, &conn)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, comment2.id);

        Ok(())
    }

    #[test]
    fn select_latest() -> Result<()> {
        let conn = conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;
        let time1 = OffsetDateTime::now_utc().saturating_sub(Duration::hours(1));
        let comment1 = super::insert(1, "First", &conn)?;
        let _comment1 = super::set_updated_at(comment1.id, time1, &conn)?;

        let time2 = OffsetDateTime::now_utc();
        let comment2 = super::insert(1, "Second", &conn)?;
        let comment2 = super::set_updated_at(comment2.id, time2, &conn)?;

        // Test latest
        let results = super::select_latest(1, &conn)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, comment2.id);

        Ok(())
    }

    #[test]
    fn select_created_between() -> Result<()> {
        let conn = conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;

        let time1 = OffsetDateTime::now_utc().saturating_sub(Duration::hours(1));
        let comment1 = super::insert(1, "First", &conn)?;
        let comment1 = super::set_created_at(comment1.id, time1, &conn)?;

        let time2 = OffsetDateTime::now_utc();
        let comment2 = super::insert(1, "Second", &conn)?;
        let _comment2 = super::set_created_at(comment2.id, time2, &conn)?;

        // Test created_between
        let results = super::select_created_between(
            &(time1 - time::Duration::seconds(1)),
            &(time1 + time::Duration::seconds(1)),
            &conn,
        )?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], comment1);

        Ok(())
    }

    #[test]
    fn select_by_element_id() -> Result<()> {
        let conn = conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;
        let comment = super::insert(1, "First", &conn)?;

        // Test select_by_element_id
        let results = super::select_by_element_id(1, false, 10, &conn)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], comment);

        Ok(())
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;

        let comment = super::insert(1, "Test", &conn)?;

        // Test setting deleted_at
        let deleted_time = OffsetDateTime::now_utc();
        let comment = super::set_deleted_at(comment.id, Some(deleted_time), &conn)?;
        assert_eq!(comment.deleted_at, Some(deleted_time));

        // Test including deleted items
        let results_with_deleted = super::select_by_element_id(1, true, 10, &conn)?;
        assert_eq!(results_with_deleted.len(), 1);

        // Test excluding deleted items
        let results_without_deleted = super::select_by_element_id(1, false, 10, &conn)?;
        assert_eq!(results_without_deleted.len(), 0);

        // Test un-deleting
        let comment = super::set_deleted_at(comment.id, None, &conn)?;
        assert_eq!(comment.deleted_at, None);

        Ok(())
    }
}
