use super::schema::{self, OgImage};
use crate::Result;
use rusqlite::{params, Connection};
use schema::Columns::*;
use schema::TABLE_NAME as TABLE;

pub fn insert(
    element_id: i64,
    version: i64,
    image_data: Vec<u8>,
    width: i64,
    height: i64,
    size_bytes: i64,
    conn: &Connection,
) -> Result<OgImage> {
    let sql = format!(
        r#"
            INSERT INTO {TABLE} ({ElementId}, {Version}, {ImageData}, {Width}, {Height}, {SizeBytes})
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            RETURNING {projection}
        "#,
        projection = OgImage::projection(),
    );
    conn.query_row(
        &sql,
        params![element_id, version, image_data, width, height, size_bytes],
        OgImage::mapper(),
    )
    .map_err(Into::into)
}

pub fn select_by_element_id(element_id: i64, conn: &Connection) -> Result<Option<OgImage>> {
    match conn
        .prepare(&format!(
            r#"
                SELECT {projection}
                FROM {TABLE}
                WHERE {ElementId} = ?1
            "#,
            projection = OgImage::projection(),
        ))?
        .query_row(params![element_id], OgImage::mapper())
    {
        Ok(og_image) => Ok(Some(og_image)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn select_all_with_zero_metadata(conn: &Connection) -> Result<Vec<OgImage>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {TABLE}
            WHERE {Width} = 0 AND {Height} = 0 AND {SizeBytes} = 0
        "#,
        projection = OgImage::projection(),
    );
    conn.prepare(&sql)?
        .query_map(params![], OgImage::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn update_metadata(
    element_id: i64,
    width: i64,
    height: i64,
    size_bytes: i64,
    conn: &Connection,
) -> Result<usize> {
    let sql = format!(
        r#"
            UPDATE {TABLE}
            SET {Width} = ?2, {Height} = ?3, {SizeBytes} = ?4
            WHERE {ElementId} = ?1
        "#
    );
    conn.execute(&sql, params![element_id, width, height, size_bytes])
        .map_err(Into::into)
}

pub fn delete(element_id: i64, conn: &Connection) -> Result<usize> {
    conn.execute(
        &format!(
            r#"
                DELETE FROM {TABLE}
                WHERE {ElementId} = ?1
            "#
        ),
        params![element_id],
    )
    .map_err(Into::into)
}

#[cfg(test)]
mod test {
    use super::super::super::test::conn;
    use crate::Result;

    #[test]
    fn insert() -> Result<()> {
        let conn = conn();
        let element_id = 1;
        let version = 1;
        let image_data = vec![1, 2, 3, 4, 5];
        let width = 600;
        let height = 315;
        let size_bytes = image_data.len() as i64;

        let _inserted = super::insert(
            element_id,
            version,
            image_data.clone(),
            width,
            height,
            size_bytes,
            &conn,
        )?;
        let selected = super::select_by_element_id(element_id, &conn)?;

        assert!(selected.is_some());
        let selected = selected.unwrap();
        assert_eq!(selected.element_id, element_id);
        assert_eq!(selected.version, version);
        assert_eq!(selected.image_data, image_data);
        assert_eq!(selected.width, width);
        assert_eq!(selected.height, height);
        assert_eq!(selected.size_bytes, size_bytes);
        assert!(selected.created_at > time::OffsetDateTime::UNIX_EPOCH);

        Ok(())
    }

    #[test]
    fn select_by_element_id_exists() -> Result<()> {
        let conn = conn();
        let element_id = 1;
        let version = 1;
        let image_data = vec![1, 2, 3];

        super::insert(element_id, version, image_data, 600, 315, 3, &conn)?;
        let res = super::select_by_element_id(element_id, &conn)?;

        assert!(res.is_some());
        assert_eq!(res.unwrap().element_id, element_id);

        Ok(())
    }

    #[test]
    fn select_by_element_id_not_exists() -> Result<()> {
        let conn = conn();
        let res = super::select_by_element_id(9999i64, &conn)?;

        assert!(res.is_none());

        Ok(())
    }

    #[test]
    fn delete() -> Result<()> {
        let conn = conn();
        let element_id = 1i64;
        let version = 1i64;
        let image_data = vec![1, 2, 3];

        super::insert(element_id, version, image_data, 600, 315, 3, &conn)?;
        let before_delete = super::select_by_element_id(element_id, &conn)?;
        assert!(before_delete.is_some());

        super::delete(element_id, &conn)?;
        let after_delete = super::select_by_element_id(element_id, &conn)?;
        assert!(after_delete.is_none());

        Ok(())
    }

    #[test]
    fn select_all_with_zero_metadata_and_update_metadata() -> Result<()> {
        let conn = conn();

        super::insert(1, 1, vec![1, 2, 3, 4], 0, 0, 0, &conn)?;
        super::insert(2, 1, vec![5, 6, 7, 8, 9], 0, 0, 0, &conn)?;
        super::insert(3, 1, vec![10, 11, 12, 13, 14, 15], 600, 315, 6, &conn)?;

        let pending = super::select_all_with_zero_metadata(&conn)?;
        assert_eq!(pending.len(), 2);
        let pending_ids: Vec<i64> = pending.iter().map(|it| it.element_id).collect();
        assert!(pending_ids.contains(&1));
        assert!(pending_ids.contains(&2));

        for row in &pending {
            super::update_metadata(row.element_id, 600, 315, row.image_data.len() as i64, &conn)?;
        }

        let pending = super::select_all_with_zero_metadata(&conn)?;
        assert!(pending.is_empty());

        let updated = super::select_by_element_id(1, &conn)?.unwrap();
        assert_eq!(updated.width, 600);
        assert_eq!(updated.height, 315);
        assert_eq!(updated.size_bytes, 4);
        let updated = super::select_by_element_id(2, &conn)?.unwrap();
        assert_eq!(updated.width, 600);
        assert_eq!(updated.height, 315);
        assert_eq!(updated.size_bytes, 5);
        let untouched = super::select_by_element_id(3, &conn)?.unwrap();
        assert_eq!(untouched.width, 600);
        assert_eq!(untouched.height, 315);
        assert_eq!(untouched.size_bytes, 6);

        Ok(())
    }
}
