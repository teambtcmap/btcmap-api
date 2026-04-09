use super::schema::{self, OgImage};
use crate::Result;
use rusqlite::{params, Connection};
use schema::Columns::*;
use schema::TABLE_NAME as TABLE;

pub fn insert(
    element_id: i64,
    version: i64,
    image_data: Vec<u8>,
    conn: &Connection,
) -> Result<OgImage> {
    let sql = format!(
        r#"
            INSERT INTO {TABLE} ({ElementId}, {Version}, {ImageData})
            VALUES (?1, ?2, ?3)
            RETURNING {projection}
        "#,
        projection = OgImage::projection(),
    );
    conn.query_row(
        &sql,
        params![element_id, version, image_data],
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

        let _inserted = super::insert(element_id, version, image_data.clone(), &conn)?;
        let selected = super::select_by_element_id(element_id, &conn)?;

        assert!(selected.is_some());
        let selected = selected.unwrap();
        assert_eq!(selected.element_id, element_id);
        assert_eq!(selected.version, version);
        assert_eq!(selected.image_data, image_data);
        assert!(selected.created_at > time::OffsetDateTime::UNIX_EPOCH);

        Ok(())
    }

    #[test]
    fn select_by_element_id_exists() -> Result<()> {
        let conn = conn();
        let element_id = 1;
        let version = 1;
        let image_data = vec![1, 2, 3];

        super::insert(element_id, version, image_data, &conn)?;
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

        super::insert(element_id, version, image_data, &conn)?;
        let before_delete = super::select_by_element_id(element_id, &conn)?;
        assert!(before_delete.is_some());

        super::delete(element_id, &conn)?;
        let after_delete = super::select_by_element_id(element_id, &conn)?;
        assert!(after_delete.is_none());

        Ok(())
    }
}
