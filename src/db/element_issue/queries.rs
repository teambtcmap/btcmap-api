use super::schema::{self, Columns, ElementIssue, SelectOrderedBySeverityRow};
use crate::Result;
use rusqlite::{named_params, params, Connection};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn insert(
    element_id: i64,
    code: impl Into<String>,
    severity: i64,
    conn: &Connection,
) -> Result<ElementIssue> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({element_id}, {code}, {severity})
            VALUES (:element_id, :code, :severity)
            RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        element_id = Columns::ElementId.as_str(),
        code = Columns::Code.as_str(),
        severity = Columns::Severity.as_str(),
        projection = ElementIssue::projection(),
    );
    let params = named_params! {
        ":element_id": element_id,
        ":code": code.into(),
        ":severity": severity
    };
    conn.query_row(&sql, params, ElementIssue::mapper())
        .map_err(Into::into)
}

pub fn select_by_element_id(element_id: i64, conn: &Connection) -> Result<Vec<ElementIssue>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {element_id} = ?1
            ORDER BY {updated_at}, {id}
        "#,
        projection = ElementIssue::projection(),
        table = schema::TABLE_NAME,
        element_id = Columns::ElementId.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(params![element_id], ElementIssue::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_updated_since(
    updated_since: &OffsetDateTime,
    limit: Option<i64>,
    conn: &Connection,
) -> Result<Vec<ElementIssue>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {updated_at} > ?1
            ORDER BY {updated_at}, {id}
            LIMIT ?2
        "#,
        projection = ElementIssue::projection(),
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(
            params![updated_since.format(&Rfc3339)?, limit.unwrap_or(i64::MAX),],
            ElementIssue::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_ordered_by_severity(
    area_id: i64,
    limit: i64,
    offset: i64,
    conn: &Connection,
) -> Result<Vec<SelectOrderedBySeverityRow>> {
    let area_join = match area_id {
            662 => "".into(),
            _ => format!("INNER JOIN area_element ae ON ae.element_id = ei.element_id AND ae.area_id = {area_id}")
        };
    let sql = format!(
        r#"
                SELECT json_extract(e.overpass_data, '$.type'), json_extract(e.overpass_data, '$.id'), json_extract(e.overpass_data, '$.tags.name'), ei.{code}
                FROM {table} ei join element e ON e.id = ei.{element_id} {area_join}
                WHERE ei.{deleted_at} IS NULL
                ORDER BY ei.{severity} DESC
                LIMIT :limit
                OFFSET :offset;
            "#,
        code = Columns::Code.as_str(),
        table = schema::TABLE_NAME,
        element_id = Columns::ElementId.as_str(),
        deleted_at = Columns::DeletedAt.as_str(),
        severity = Columns::Severity.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(
            named_params! {
                ":limit": limit,
                ":offset": offset,
            },
            SelectOrderedBySeverityRow::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<ElementIssue> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = ElementIssue::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![id], ElementIssue::mapper())
        .map_err(Into::into)
}

pub fn select_count(area_id: i64, include_deleted: bool, conn: &Connection) -> Result<i64> {
    let area_join = match area_id {
            662 => "".into(),
            _ => format!("INNER JOIN area_element ae ON ae.element_id = ei.element_id AND ae.area_id = {area_id}")
        };
    let sql = if include_deleted {
        format!(
            r#"
                SELECT count(ei.{id})
                FROM {table} ei {area_join}
            "#,
            id = Columns::Id.as_str(),
            table = schema::TABLE_NAME,
        )
    } else {
        format!(
            r#"
                SELECT count(ei.{id})
                FROM {table} ei {area_join}
                WHERE ei.{deleted_at} IS NULL
            "#,
            id = Columns::Id.as_str(),
            table = schema::TABLE_NAME,
            deleted_at = Columns::DeletedAt.as_str(),
        )
    };
    let res: rusqlite::Result<i64, _> = conn.query_row(&sql, [], |row| row.get(0));
    res.map_err(Into::into)
}

pub fn set_severity(id: i64, severity: i64, conn: &Connection) -> Result<ElementIssue> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {severity} = ?2
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        severity = Columns::Severity.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![id, severity,])?;
    select_by_id(id, conn)
}

pub fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    conn: &Connection,
) -> Result<ElementIssue> {
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
            conn.execute(&sql, params![id, deleted_at.format(&Rfc3339)?,])?;
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

        let element_id = 123;
        let code = "test_code";
        let severity = 5;

        // Test insert
        let inserted = super::insert(element_id, code, severity, &conn)?;
        assert_eq!(inserted.element_id, element_id);
        assert_eq!(inserted.code, code);
        assert_eq!(inserted.severity, severity);

        // Test select_by_id
        let selected = super::select_by_id(inserted.id, &conn)?;
        assert_eq!(selected, inserted);
        Ok(())
    }

    #[test]
    fn select_by_element_id() -> Result<()> {
        let conn = conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;

        let element_id = 123;

        // Insert multiple issues for the same element
        let issue1 = super::insert(element_id, "code1", 1, &conn)?;
        let issue2 = super::insert(element_id, "code2", 2, &conn)?;

        // Insert issue for different element
        super::insert(456, "code3", 3, &conn)?;

        let selected = super::select_by_element_id(element_id, &conn)?;
        assert_eq!(selected.len(), 2);
        assert!(selected.contains(&issue1));
        assert!(selected.contains(&issue2));
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;

        // Insert issues with different timestamps
        let _issue1 = super::insert(1, "code1", 1, &conn)?;
        let _issue2 = super::insert(2, "code2", 2, &conn)?;

        let selected = super::select_updated_since(
            &OffsetDateTime::now_utc().saturating_add(Duration::minutes(1)),
            None,
            &conn,
        )?;
        assert!(selected.is_empty());

        // Should return both issues if we query since beginning of time
        let beginning_of_time = OffsetDateTime::UNIX_EPOCH;
        let selected = super::select_updated_since(&beginning_of_time, None, &conn)?;
        assert_eq!(selected.len(), 2);

        // Test limit
        let selected = super::select_updated_since(&beginning_of_time, Some(1), &conn)?;
        assert_eq!(selected.len(), 1);
        Ok(())
    }

    #[test]
    fn select_count() -> Result<()> {
        let conn = conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;

        // Insert some issues
        super::insert(1, "code1", 1, &conn)?;
        super::insert(2, "code2", 2, &conn)?;
        let deleted_issue = super::insert(3, "code3", 3, &conn)?;

        // Delete one issue
        super::set_deleted_at(deleted_issue.id, Some(OffsetDateTime::now_utc()), &conn)?;

        // Test count without deleted
        let count = super::select_count(662, false, &conn)?;
        assert_eq!(count, 2);

        // Test count with deleted
        let count = super::select_count(662, true, &conn)?;
        assert_eq!(count, 3);
        Ok(())
    }

    #[test]
    fn set_severity() -> Result<()> {
        let conn = conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;
        let issue = super::insert(1, "code1", 1, &conn)?;

        let updated = super::set_severity(issue.id, 5, &conn)?;
        assert_eq!(updated.severity, 5);

        let fetched = super::select_by_id(issue.id, &conn)?;
        assert_eq!(fetched.severity, 5);
        Ok(())
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;
        let issue = super::insert(1, "code1", 1, &conn).unwrap();
        let now = OffsetDateTime::now_utc();

        // Set deleted_at
        let updated = super::set_deleted_at(issue.id, Some(now), &conn).unwrap();
        assert!(updated.deleted_at.is_some());

        // Clear deleted_at
        let updated = super::set_deleted_at(issue.id, None, &conn).unwrap();
        assert!(updated.deleted_at.is_none());
        Ok(())
    }
}
