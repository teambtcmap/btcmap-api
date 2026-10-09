use super::schema::{self, PlaceImage, PlaceImageMeta};
use crate::Result;
use rusqlite::{params, Connection};
use schema::Columns::*;
use schema::TABLE_NAME as TABLE;

pub struct InsertArgs {
    pub place_id: i64,
    pub r#type: String,
    pub image_data: Vec<u8>,
    pub width: i64,
    pub height: i64,
    pub size_bytes: i64,
    pub created_by: Option<i64>,
}

pub fn insert(args: &InsertArgs, conn: &Connection) -> Result<PlaceImage> {
    let sql = format!(
        r#"
            INSERT INTO {TABLE} ({PlaceId}, {Type}, {ImageData}, {Width}, {Height}, {SizeBytes}, {CreatedBy})
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            RETURNING {projection}
        "#,
        projection = PlaceImage::projection(),
    );
    conn.query_row(
        &sql,
        params![
            args.place_id,
            &args.r#type,
            &args.image_data,
            args.width,
            args.height,
            args.size_bytes,
            args.created_by
        ],
        PlaceImage::mapper(),
    )
    .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<PlaceImage> {
    conn.query_row(
        &format!(
            r#"
                SELECT {projection}
                FROM {TABLE}
                WHERE {Id} = ?1
            "#,
            projection = PlaceImage::projection(),
        ),
        params![id],
        PlaceImage::mapper(),
    )
    .map_err(Into::into)
}

pub fn select_meta_by_id(id: i64, conn: &Connection) -> Result<PlaceImageMeta> {
    conn.query_row(
        &format!(
            r#"
                SELECT {projection}
                FROM {TABLE}
                WHERE {Id} = ?1
            "#,
            projection = PlaceImageMeta::projection(),
        ),
        params![id],
        PlaceImageMeta::mapper(),
    )
    .map_err(Into::into)
}

pub fn select_by_place_id(place_id: i64, conn: &Connection) -> Result<Vec<PlaceImageMeta>> {
    conn.prepare(&format!(
        r#"
            SELECT {projection}
            FROM {TABLE}
            WHERE {PlaceId} = ?1
            ORDER BY {Id} DESC
        "#,
        projection = PlaceImageMeta::projection(),
    ))?
    .query_map(params![place_id], PlaceImageMeta::mapper())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(Into::into)
}

pub fn select_by_place_id_and_type(
    place_id: i64,
    r#type: &str,
    conn: &Connection,
) -> Result<Vec<PlaceImageMeta>> {
    conn.prepare(&format!(
        r#"
            SELECT {projection}
            FROM {TABLE}
            WHERE {PlaceId} = ?1 AND {Type} = ?2
            ORDER BY {Id} DESC
        "#,
        projection = PlaceImageMeta::projection(),
    ))?
    .query_map(params![place_id, r#type], PlaceImageMeta::mapper())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(Into::into)
}

pub fn select_by_created_by(created_by: i64, conn: &Connection) -> Result<Vec<PlaceImageMeta>> {
    conn.prepare(&format!(
        r#"
            SELECT {projection}
            FROM {TABLE}
            WHERE {CreatedBy} = ?1
            ORDER BY {Id} DESC
        "#,
        projection = PlaceImageMeta::projection(),
    ))?
    .query_map(params![created_by], PlaceImageMeta::mapper())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(Into::into)
}

/// Newest images across all places, capped at `limit`. Ordered by upload time
/// rather than row id so it reflects the actual "recently added" order even if
/// rows were inserted out of order; `Id` breaks ties created within the same
/// millisecond.
pub fn select_recent(limit: i64, conn: &Connection) -> Result<Vec<PlaceImageMeta>> {
    conn.prepare(&format!(
        r#"
            SELECT {projection}
            FROM {TABLE}
            ORDER BY {CreatedAt} DESC, {Id} DESC
            LIMIT ?1
        "#,
        projection = PlaceImageMeta::projection(),
    ))?
    .query_map(params![limit], PlaceImageMeta::mapper())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(Into::into)
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
    use super::InsertArgs;
    use crate::Result;

    fn insert(place_id: i64, r#type: &str, conn: &rusqlite::Connection) -> Result<i64> {
        let data = vec![1, 2, 3, 4, 5];
        let size = data.len() as i64;
        let args = InsertArgs {
            place_id,
            r#type: r#type.to_string(),
            image_data: data,
            width: 600,
            height: 315,
            size_bytes: size,
            created_by: None,
        };
        Ok(super::insert(&args, conn)?.id)
    }

    #[test]
    fn insert_and_select_by_id() -> Result<()> {
        let conn = conn();
        let place_id = 42;
        let image_data = vec![1, 2, 3, 4, 5];
        let size_bytes = image_data.len() as i64;

        let args = InsertArgs {
            place_id,
            r#type: "report".to_string(),
            image_data: image_data.clone(),
            width: 600,
            height: 315,
            size_bytes,
            created_by: Some(7),
        };
        let inserted = super::insert(&args, &conn)?;
        assert_eq!(inserted.place_id, place_id);
        assert_eq!(inserted.r#type, "report");
        assert_eq!(inserted.image_data, image_data);
        assert_eq!(inserted.width, 600);
        assert_eq!(inserted.height, 315);
        assert_eq!(inserted.size_bytes, size_bytes);
        assert_eq!(inserted.created_by, Some(7));
        assert!(inserted.created_at > time::OffsetDateTime::UNIX_EPOCH);

        let selected = super::select_by_id(inserted.id, &conn)?;
        assert_eq!(selected.id, inserted.id);
        assert_eq!(selected.image_data, image_data);
        assert_eq!(selected.created_by, Some(7));

        Ok(())
    }

    #[test]
    fn select_by_place_id_returns_all_types() -> Result<()> {
        let conn = conn();
        insert(1, "report", &conn)?;
        insert(1, "cover", &conn)?;
        insert(2, "report", &conn)?;

        let res = super::select_by_place_id(1, &conn)?;
        assert_eq!(res.len(), 2);
        assert!(res.iter().all(|it| it.place_id == 1));

        Ok(())
    }

    #[test]
    fn select_by_place_id_and_type_filters() -> Result<()> {
        let conn = conn();
        let first = insert(1, "report", &conn)?;
        insert(1, "cover", &conn)?;
        let second = insert(1, "report", &conn)?;

        let res = super::select_by_place_id_and_type(1, "report", &conn)?;
        let ids: Vec<i64> = res.iter().map(|it| it.id).collect();
        // newest first
        assert_eq!(vec![second, first], ids);

        Ok(())
    }

    #[test]
    fn select_by_place_id_and_type_empty() -> Result<()> {
        let conn = conn();
        let res = super::select_by_place_id_and_type(9999, "report", &conn)?;
        assert!(res.is_empty());
        Ok(())
    }

    fn insert_with_created_by(
        place_id: i64,
        r#type: &str,
        created_by: Option<i64>,
        conn: &rusqlite::Connection,
    ) -> Result<i64> {
        let data = vec![1, 2, 3, 4, 5];
        let size = data.len() as i64;
        let args = InsertArgs {
            place_id,
            r#type: r#type.to_string(),
            image_data: data,
            width: 600,
            height: 315,
            size_bytes: size,
            created_by,
        };
        Ok(super::insert(&args, conn)?.id)
    }

    #[test]
    fn select_meta_by_id_omits_bytes() -> Result<()> {
        let conn = conn();
        let inserted = super::insert(
            &InsertArgs {
                place_id: 42,
                r#type: "report".to_string(),
                image_data: vec![1, 2, 3, 4, 5],
                width: 600,
                height: 315,
                size_bytes: 5,
                created_by: Some(7),
            },
            &conn,
        )?;

        let meta = super::select_meta_by_id(inserted.id, &conn)?;
        assert_eq!(meta.id, inserted.id);
        assert_eq!(meta.place_id, 42);
        assert_eq!(meta.r#type, "report");
        assert_eq!(meta.width, 600);
        assert_eq!(meta.height, 315);
        assert_eq!(meta.size_bytes, 5);
        assert_eq!(meta.created_by, Some(7));
        Ok(())
    }

    #[test]
    fn select_meta_by_id_errors_for_unknown() {
        let conn = conn();
        assert!(super::select_meta_by_id(9999, &conn).is_err());
    }

    #[test]
    fn select_by_created_by_returns_only_owned_images() -> Result<()> {
        let conn = conn();
        let first = insert_with_created_by(1, "user", Some(7), &conn)?;
        insert_with_created_by(1, "report", Some(8), &conn)?;
        let second = insert_with_created_by(2, "user", Some(7), &conn)?;
        insert_with_created_by(1, "user", None, &conn)?;

        let res = super::select_by_created_by(7, &conn)?;
        let ids: Vec<i64> = res.iter().map(|it| it.id).collect();
        // newest first, and never other users' or unattributed images
        assert_eq!(vec![second, first], ids);
        Ok(())
    }

    #[test]
    fn select_by_created_by_empty() -> Result<()> {
        let conn = conn();
        let res = super::select_by_created_by(9999, &conn)?;
        assert!(res.is_empty());
        Ok(())
    }

    #[test]
    fn select_recent_orders_newest_first_and_limits() -> Result<()> {
        let conn = conn();
        let first = insert(1, "report", &conn)?;
        let second = insert(2, "user", &conn)?;
        let third = insert(1, "cover", &conn)?;

        let res = super::select_recent(10, &conn)?;
        let ids: Vec<i64> = res.iter().map(|it| it.id).collect();
        assert_eq!(vec![third, second, first], ids);

        let limited = super::select_recent(2, &conn)?;
        let ids: Vec<i64> = limited.iter().map(|it| it.id).collect();
        assert_eq!(vec![third, second], ids);

        Ok(())
    }

    #[test]
    fn select_recent_empty() -> Result<()> {
        let conn = conn();
        assert!(super::select_recent(10, &conn)?.is_empty());
        Ok(())
    }

    #[test]
    fn delete_removes_row() -> Result<()> {
        let conn = conn();
        let id = insert(1, "report", &conn)?;

        super::delete(id, &conn)?;

        assert!(super::select_by_id(id, &conn).is_err());
        assert!(super::select_by_place_id(1, &conn)?.is_empty());

        Ok(())
    }
}
