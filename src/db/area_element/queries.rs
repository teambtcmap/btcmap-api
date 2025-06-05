use super::schema::{self, AreaElement, Columns};
use crate::Result;
use rusqlite::{params, Connection};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn insert(area_id: i64, element_id: i64, conn: &Connection) -> Result<AreaElement> {
    let sql = format!(
        r#"
            INSERT INTO {table} (
                {area_id},
                {element_id}
            ) VALUES (
                ?1,
                ?2
            )
        "#,
        table = schema::TABLE_NAME,
        area_id = Columns::AreaId.as_str(),
        element_id = Columns::ElementId.as_str(),
    );
    conn.execute(&sql, params![area_id, element_id,])?;
    select_by_id(conn.last_insert_rowid(), conn)
}

pub fn select_updated_since(
    updated_since: &OffsetDateTime,
    limit: Option<i64>,
    conn: &Connection,
) -> Result<Vec<AreaElement>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {updated_at} > ?1
            ORDER BY {updated_at}, {id}
            LIMIT ?2
        "#,
        projection = AreaElement::projection(),
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(
            params![updated_since.format(&Rfc3339)?, limit.unwrap_or(i64::MAX),],
            AreaElement::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_area_id(area_id: i64, conn: &Connection) -> Result<Vec<AreaElement>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {area_id} = ?1
            ORDER BY {updated_at}, {id}
        "#,
        projection = AreaElement::projection(),
        table = schema::TABLE_NAME,
        area_id = Columns::AreaId.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(params![area_id,], AreaElement::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_element_id(element_id: i64, conn: &Connection) -> Result<Vec<AreaElement>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {element_id} = ?1
            ORDER BY {updated_at}, {id}
        "#,
        projection = AreaElement::projection(),
        table = schema::TABLE_NAME,
        element_id = Columns::ElementId.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(params![element_id,], AreaElement::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<AreaElement> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = AreaElement::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![id], AreaElement::mapper())
        .map_err(Into::into)
}

#[cfg(test)]
pub fn set_updated_at(
    id: i64,
    updated_at: &OffsetDateTime,
    conn: &Connection,
) -> Result<AreaElement> {
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
    conn.execute(&sql, params![id, updated_at.format(&Rfc3339)?,])?;
    select_by_id(id, conn)
}

pub fn set_deleted_at(
    id: i64,
    deleted_at: Option<&OffsetDateTime>,
    conn: &Connection,
) -> Result<AreaElement> {
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
                    WHERE {id} = ?
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
mod tests {
    use time::{Duration, OffsetDateTime};

    use crate::{error::Error, test::mock_conn, Result};

    #[test]
    fn insert() -> Result<()> {
        // Setup in-memory database
        let conn = mock_conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;

        // Test data
        let test_area_id = 42;
        let test_element_id = 123;

        // Execute insert
        let item = super::insert(test_area_id, test_element_id, &conn)?;

        // Verify the returned struct has correct values
        assert_eq!(item.area_id, test_area_id);
        assert_eq!(item.element_id, test_element_id);

        // Verify the data was actually inserted in the database
        let db_item = super::select_by_id(item.id, &conn)?;
        assert_eq!(db_item.area_id, test_area_id);
        assert_eq!(db_item.element_id, test_element_id);

        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = mock_conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;

        // Create test timestamps
        let base_time = OffsetDateTime::now_utc();
        let time1 = base_time - Duration::days(2);
        let time2 = base_time - Duration::days(1);
        let time3 = base_time;

        // Insert test data
        let _area1 = super::insert(1, 10, &conn)?;
        let _area1 = super::set_updated_at(_area1.id, &time1, &conn)?;
        let _area2 = super::insert(2, 20, &conn)?;
        let _area2 = super::set_updated_at(_area2.id, &time2, &conn)?;
        let _area3 = super::insert(3, 30, &conn)?;
        let _area3 = super::set_updated_at(_area3.id, &time3, &conn)?;

        // Test 1: Select records updated after time1 (should return time2 and time3)
        let result = super::select_updated_since(&time1, None, &conn)?;
        assert_eq!(result.len(), 2);
        // Ordered by updated_at then id
        assert_eq!(result[0].id, 2);
        assert_eq!(result[1].id, 3);

        // Test 2: Select records updated after time2 (should return time3 only)
        let result = super::select_updated_since(&time2, None, &conn)?;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 3);

        // Test 3: Select records updated after time3 (should return empty)
        let result = super::select_updated_since(&time3, None, &conn)?;
        assert_eq!(result.len(), 0);

        // Test 4: Test limit parameter
        // Add another record with same timestamp
        let _area4 = super::insert(4, 40, &conn)?;
        let _area4 = super::set_updated_at(_area4.id, &time3, &conn);
        let result = super::select_updated_since(&time1, Some(1), &conn)?;
        assert_eq!(result.len(), 1);
        // Should return the first record by ordering
        assert_eq!(result[0].id, 2);

        Ok(())
    }

    #[test]
    fn select_updated_since_empty_db() -> Result<()> {
        let conn = mock_conn();
        let now = OffsetDateTime::now_utc();
        let result = super::select_updated_since(&now, None, &conn)?;
        assert_eq!(result.len(), 0);
        Ok(())
    }

    #[test]
    fn select_updated_since_ordering() -> Result<()> {
        let conn = mock_conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;
        let time = OffsetDateTime::now_utc();

        // Insert records with same updated_at but different ids
        let _area1 = super::insert(2, 20, &conn)?;
        let _area1 = super::set_updated_at(_area1.id, &time, &conn)?;
        let _area2 = super::insert(1, 10, &conn)?;
        let _area2 = super::set_updated_at(_area2.id, &time, &conn)?;
        let _area3 = super::insert(3, 30, &conn)?;
        let _area3 = super::set_updated_at(_area3.id, &time, &conn)?;

        let result = super::select_updated_since(&(time - Duration::seconds(1)), None, &conn)?;
        assert_eq!(result.len(), 3);
        // Should be ordered by id
        assert_eq!(result[0].id, 1);
        assert_eq!(result[1].id, 2);
        assert_eq!(result[2].id, 3);

        Ok(())
    }

    #[test]
    fn select_by_area_id_basic() -> Result<()> {
        let conn = mock_conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;
        let now = OffsetDateTime::now_utc();

        // Insert test data for multiple areas
        let _item1 = super::insert(1, 1, &conn)?; // area 1
        let _item1 = super::set_updated_at(_item1.id, &(now - Duration::days(2)), &conn)?;
        let _item2 = super::insert(2, 2, &conn)?; // area 2
        let _item2 = super::set_updated_at(_item2.id, &(now - Duration::days(1)), &conn)?;
        let _item3 = super::insert(1, 3, &conn)?; // area 1
        let _item3 = super::set_updated_at(_item3.id, &(now - Duration::hours(1)), &conn)?;
        let _item4 = super::insert(2, 4, &conn)?; // area 2
        let _item4 = super::set_updated_at(_item4.id, &now, &conn)?;

        // Test for area_id = 1
        let results = super::select_by_area_id(1, &conn)?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 1); // Older updated_at comes first
        assert_eq!(results[1].id, 3);

        // Test for area_id = 2
        let results = super::select_by_area_id(2, &conn)?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 2);
        assert_eq!(results[1].id, 4);

        // Test for non-existent area_id
        let results = super::select_by_area_id(99, &conn)?;
        assert_eq!(results.len(), 0);

        Ok(())
    }

    #[test]
    fn select_by_area_id_ordering() -> Result<()> {
        let conn = mock_conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;
        let now = OffsetDateTime::now_utc();

        // Insert records with same area_id and same updated_at but different ids
        super::insert(1, 1, &conn)?;
        super::set_updated_at(1, &now, &conn)?;
        super::insert(1, 2, &conn)?;
        super::set_updated_at(2, &now, &conn)?;
        super::insert(1, 3, &conn)?;
        super::set_updated_at(3, &now, &conn)?;

        let results = super::select_by_area_id(1, &conn)?;
        assert_eq!(results.len(), 3);
        // Should be ordered by id since updated_at is the same
        assert_eq!(results[0].id, 1);
        assert_eq!(results[1].id, 2);
        assert_eq!(results[2].id, 3);

        Ok(())
    }

    #[test]
    fn select_by_area_id_empty_db() -> Result<()> {
        let conn = mock_conn();
        let results = super::select_by_area_id(10, &conn)?;
        assert_eq!(results.len(), 0);
        Ok(())
    }

    #[test]
    fn select_by_element_id_basic() -> Result<()> {
        let conn = mock_conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;
        let now = OffsetDateTime::now_utc();

        // Insert test data for multiple elements
        let _item1 = super::insert(1, 1, &conn)?; // element 1
        let _item1 = super::set_updated_at(_item1.id, &(now - Duration::days(2)), &conn)?;
        let _item2 = super::insert(2, 2, &conn)?; // element 2
        let _item2 = super::set_updated_at(_item2.id, &(now - Duration::days(1)), &conn)?;
        let _item3 = super::insert(3, 1, &conn)?; // element 1
        let _item3 = super::set_updated_at(_item3.id, &(now - Duration::hours(1)), &conn)?;
        let _item4 = super::insert(4, 2, &conn)?; // element 2
        let _item4 = super::set_updated_at(_item4.id, &now, &conn)?;

        // Test for element_id = 1
        let results = super::select_by_element_id(1, &conn)?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 1); // Older updated_at comes first
        assert_eq!(results[1].id, 3);

        // Test for element_id = 2
        let results = super::select_by_element_id(2, &conn)?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 2);
        assert_eq!(results[1].id, 4);

        // Test for non-existent element_id
        let results = super::select_by_element_id(99, &conn)?;
        assert_eq!(results.len(), 0);

        Ok(())
    }

    #[test]
    fn select_by_element_id_ordering() -> Result<()> {
        let conn = mock_conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;
        let now = OffsetDateTime::now_utc();

        // Insert records with same element_id and same updated_at but different ids
        super::insert(1, 1, &conn)?;
        super::set_updated_at(1, &now, &conn)?;
        super::insert(2, 1, &conn)?;
        super::set_updated_at(1, &now, &conn)?;
        super::insert(3, 1, &conn)?;
        super::set_updated_at(1, &now, &conn)?;

        let results = super::select_by_element_id(1, &conn)?;
        assert_eq!(results.len(), 3);
        // Should be ordered by id since updated_at is the same
        assert_eq!(results[0].id, 1);
        assert_eq!(results[1].id, 2);
        assert_eq!(results[2].id, 3);

        Ok(())
    }

    #[test]
    fn select_by_element_id_empty_db() -> Result<()> {
        let conn = mock_conn();
        let results = super::select_by_element_id(10, &conn)?;
        assert_eq!(results.len(), 0);
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        // Setup in-memory database
        let conn = mock_conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;

        // Insert test data
        let test_id = 1;
        let test_area_id = 42;
        let test_element_id = 123;
        super::insert(test_area_id, test_element_id, &conn)?;

        // Test 1: Select existing record
        let result = super::select_by_id(test_id, &conn)?;
        assert_eq!(result.id, test_id);
        assert_eq!(result.area_id, test_area_id);
        assert_eq!(result.element_id, test_element_id);

        // Test 2: Select non-existent record (should error)
        let non_existent_id = 999;
        assert!(matches!(
            super::select_by_id(non_existent_id, &conn),
            Err(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
        ));
        Ok(())
    }

    #[test]
    fn test_set_updated_at() -> Result<()> {
        // Setup test database
        let conn = mock_conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;

        // Insert test data
        let test_id = 1;
        let original_time = OffsetDateTime::now_utc();
        let item = super::insert(1, 1, &conn)?;
        let item = super::set_updated_at(item.id, &original_time, &conn)?;

        // Set new updated_at time
        let new_time = original_time + Duration::hours(1);
        let item = super::set_updated_at(item.id, &new_time, &conn)?;

        // Verify returned struct has the new time
        assert_eq!(item.updated_at, new_time);
        assert_eq!(item.id, test_id); // Other fields should remain unchanged

        // Verify database was actually updated
        let db_time = super::select_by_id(item.id, &conn)?.updated_at;
        assert_eq!(db_time, new_time);

        Ok(())
    }

    #[test]
    fn set_updated_at_nonexistent_id() {
        let conn = mock_conn();
        let res = super::set_updated_at(1, &OffsetDateTime::now_utc(), &conn);
        assert!(res.is_err());
        assert!(matches!(
            res.unwrap_err(),
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows)
        ));
    }

    #[test]
    fn set_deleted_at_with_timestamp() -> Result<()> {
        // Setup test database
        let conn = mock_conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;

        // Insert test data
        let item = super::insert(1, 1, &conn)?;

        // Set deleted_at timestamp
        let deleted_time = OffsetDateTime::now_utc();
        let item = super::set_deleted_at(item.id, Some(&deleted_time), &conn)?;

        // Verify returned struct
        assert_eq!(item.deleted_at, Some(deleted_time));
        assert_eq!(item.id, 1);

        // Verify database was updated
        let db_deleted_at = super::select_by_id(1, &conn)?.deleted_at;
        assert_eq!(db_deleted_at, Some(deleted_time));

        Ok(())
    }

    #[test]
    fn set_deleted_at_with_null() -> Result<()> {
        let conn = mock_conn();
        // Disable foreign keys for this test
        conn.pragma_update(None, "foreign_keys", &false)?;
        let deleted_time = OffsetDateTime::now_utc();

        // Insert with deleted_at already set
        let item = super::insert(1, 1, &conn)?;
        let item = super::set_deleted_at(item.id, Some(&deleted_time), &conn)?;
        assert_eq!(item.deleted_at, Some(deleted_time));

        // Set deleted_at to NULL
        let item = super::set_deleted_at(item.id, None, &conn)?;
        assert_eq!(item.deleted_at, None);

        // Verify database was updated
        let db_deleted_at = super::select_by_id(1, &conn)?.deleted_at;
        assert!(db_deleted_at.is_none());

        Ok(())
    }

    #[test]
    fn set_deleted_at_nonexistent_id() {
        let conn = mock_conn();
        let res = super::set_deleted_at(1, Some(&OffsetDateTime::now_utc()), &conn);
        assert!(res.is_err());
        assert!(matches!(
            res.unwrap_err(),
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows)
        ));
    }
}
