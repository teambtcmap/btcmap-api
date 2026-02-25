use super::schema::{self, OgImage};
use crate::Result;
use rusqlite::{params, Connection};

pub fn insert(
    element_id: i64,
    version: i64,
    image_data: Vec<u8>,
    conn: &Connection,
) -> Result<OgImage> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({element_id}, {version}, {image_data})
            VALUES (?1, ?2, ?3)
            RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        element_id = schema::Columns::ElementId.as_str(),
        version = schema::Columns::Version.as_str(),
        image_data = schema::Columns::ImageData.as_str(),
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
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {element_id} = ?1
        "#,
        projection = OgImage::projection(),
        table = schema::TABLE_NAME,
        element_id = schema::Columns::ElementId.as_str(),
    );
    let result = conn
        .prepare(&sql)?
        .query_row(params![element_id], OgImage::mapper());
    match result {
        Ok(og_image) => Ok(Some(og_image)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn delete(element_id: i64, conn: &Connection) -> Result<()> {
    let sql = format!(
        r#"
            DELETE FROM {table}
            WHERE {element_id} = ?1
        "#,
        table = schema::TABLE_NAME,
        element_id = schema::Columns::ElementId.as_str(),
    );
    conn.execute(&sql, params![element_id])?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::db::image::test::conn;
    use crate::Result;

    #[test]
    fn insert() -> Result<()> {
        let conn = conn();
        let element_id = 1i64;
        let version = 1i64;
        let image_data = vec![1, 2, 3, 4, 5];

        let _inserted = super::insert(element_id, version, image_data.clone(), &conn)?;
        let selected = super::select_by_element_id(element_id, &conn)?;

        assert!(selected.is_some());
        let selected = selected.unwrap();
        assert_eq!(selected.element_id, element_id);
        assert_eq!(selected.version, version);
        assert_eq!(selected.image_data, image_data);
        assert!(selected.created_at > time::OffsetDateTime::from_unix_timestamp(0).unwrap());

        Ok(())
    }

    #[test]
    fn select_by_element_id_exists() -> Result<()> {
        let conn = conn();
        let element_id = 1i64;
        let version = 1i64;
        let image_data = vec![1, 2, 3];

        super::insert(element_id, version, image_data, &conn)?;
        let result = super::select_by_element_id(element_id, &conn)?;

        assert!(result.is_some());
        assert_eq!(result.unwrap().element_id, element_id);

        Ok(())
    }

    #[test]
    fn select_by_element_id_not_exists() -> Result<()> {
        let conn = conn();
        let result = super::select_by_element_id(9999i64, &conn)?;

        assert!(result.is_none());

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
