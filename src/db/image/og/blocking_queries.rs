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
