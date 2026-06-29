use super::schema::{self, AreaImage};
use crate::Result;
use rusqlite::{params, Connection};
use schema::Columns::*;
use schema::TABLE_NAME as TABLE;

pub fn insert(
    area_id: i64,
    r#type: &str,
    image_data: Vec<u8>,
    width: i64,
    height: i64,
    size_bytes: i64,
    conn: &Connection,
) -> Result<AreaImage> {
    let sql = format!(
        r#"
            INSERT INTO {TABLE} ({AreaId}, {Type}, {ImageData}, {Width}, {Height}, {SizeBytes})
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            RETURNING {projection}
        "#,
        projection = AreaImage::projection(),
    );
    conn.query_row(
        &sql,
        params![area_id, r#type, image_data, width, height, size_bytes],
        AreaImage::mapper(),
    )
    .map_err(Into::into)
}

pub fn select_by_area_id(area_id: i64, conn: &Connection) -> Result<Option<AreaImage>> {
    match conn
        .prepare(&format!(
            r#"
                SELECT {projection}
                FROM {TABLE}
                WHERE {AreaId} = ?1
            "#,
            projection = AreaImage::projection(),
        ))?
        .query_row(params![area_id], AreaImage::mapper())
    {
        Ok(og_image) => Ok(Some(og_image)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn delete(id: i64, conn: &Connection) -> Result<usize> {
    conn.execute(
        &format!(
            r#"
                DELETE FROM {TABLE}
                WHERE {Id} = ?1
            "#
        ),
        params![id],
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
        let area_id = 1;
        let r#type = "test";
        let image_data = vec![1, 2, 3, 4, 5];
        let width = 600;
        let height = 315;
        let size_bytes = image_data.len() as i64;

        let _inserted = super::insert(
            area_id,
            r#type,
            image_data.clone(),
            width,
            height,
            size_bytes,
            &conn,
        )?;
        let selected = super::select_by_area_id(area_id, &conn)?;

        assert!(selected.is_some());
        let selected = selected.unwrap();
        assert_eq!(selected.area_id, area_id);
        assert_eq!(selected.r#type, r#type);
        assert_eq!(selected.image_data, image_data);
        assert_eq!(selected.width, width);
        assert_eq!(selected.height, height);
        assert_eq!(selected.size_bytes, size_bytes);
        assert!(selected.created_at > time::OffsetDateTime::UNIX_EPOCH);

        Ok(())
    }

    #[test]
    fn select_by_area_id_exists() -> Result<()> {
        let conn = conn();
        let area_id = 1;
        let image_data = vec![1, 2, 3];

        super::insert(area_id, "test", image_data, 600, 315, 3, &conn)?;
        let res = super::select_by_area_id(area_id, &conn)?;

        assert!(res.is_some());
        assert_eq!(res.unwrap().area_id, area_id);

        Ok(())
    }

    #[test]
    fn select_by_element_id_not_exists() -> Result<()> {
        let conn = conn();
        let res = super::select_by_area_id(9999i64, &conn)?;

        assert!(res.is_none());

        Ok(())
    }

    #[test]
    fn delete() -> Result<()> {
        let conn = conn();
        let area_id = 1i64;
        let image_data = vec![1, 2, 3];

        let img = super::insert(area_id, "test", image_data, 600, 315, 3, &conn)?;
        let before_delete = super::select_by_area_id(area_id, &conn)?;
        assert!(before_delete.is_some());

        super::delete(img.id, &conn)?;
        let after_delete = super::select_by_area_id(area_id, &conn)?;
        assert!(after_delete.is_none());

        Ok(())
    }
}
