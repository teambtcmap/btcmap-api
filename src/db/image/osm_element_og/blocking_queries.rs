use super::schema::{self, OsmElementOg};
use crate::Result;
use rusqlite::{named_params, params, Connection};
use schema::Columns::*;
use schema::TABLE_NAME as TABLE;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn upsert(
    osm_type: &str,
    osm_id: i64,
    image_data: Vec<u8>,
    width: i64,
    height: i64,
    size_bytes: i64,
    conn: &Connection,
) -> Result<OsmElementOg> {
    let sql = format!(
        r#"
            INSERT INTO {TABLE} ({OsmType}, {OsmId}, {ImageData}, {Width}, {Height}, {SizeBytes})
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT ({OsmType}, {OsmId}) DO UPDATE SET
                {ImageData} = excluded.{ImageData},
                {Width} = excluded.{Width},
                {Height} = excluded.{Height},
                {SizeBytes} = excluded.{SizeBytes},
                {CreatedAt} = strftime('%Y-%m-%dT%H:%M:%fZ')
            RETURNING {projection}
        "#,
        projection = OsmElementOg::projection(),
    );
    conn.query_row(
        &sql,
        params![osm_type, osm_id, image_data, width, height, size_bytes],
        OsmElementOg::mapper(),
    )
    .map_err(Into::into)
}

pub fn select_by_osm_id(
    osm_type: &str,
    osm_id: i64,
    conn: &Connection,
) -> Result<Option<OsmElementOg>> {
    match conn
        .prepare(&format!(
            r#"
                SELECT {projection}
                FROM {TABLE}
                WHERE {OsmType} = ?1 AND {OsmId} = ?2
            "#,
            projection = OsmElementOg::projection(),
        ))?
        .query_row(params![osm_type, osm_id], OsmElementOg::mapper())
    {
        Ok(og_image) => Ok(Some(og_image)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn delete_created_before(cutoff: OffsetDateTime, conn: &Connection) -> Result<usize> {
    conn.execute(
        &format!(
            r#"
                DELETE FROM {TABLE}
                WHERE {CreatedAt} < :cutoff
            "#
        ),
        named_params! { ":cutoff": cutoff.format(&Rfc3339)? },
    )
    .map_err(Into::into)
}

#[cfg(test)]
mod test {
    use super::super::super::test::conn;
    use super::schema::Columns::*;
    use super::TABLE;
    use crate::Result;
    use time::{Duration, OffsetDateTime};

    #[test]
    fn upsert_insert() -> Result<()> {
        let conn = conn();
        let osm_type = "way";
        let osm_id = 1;
        let image_data = vec![1, 2, 3, 4, 5];
        let width = 600;
        let height = 315;
        let size_bytes = image_data.len() as i64;

        let inserted = super::upsert(
            osm_type,
            osm_id,
            image_data.clone(),
            width,
            height,
            size_bytes,
            &conn,
        )?;

        assert_eq!(inserted.osm_type, osm_type);
        assert_eq!(inserted.osm_id, osm_id);
        assert_eq!(inserted.image_data, image_data);
        assert_eq!(inserted.width, width);
        assert_eq!(inserted.height, height);
        assert_eq!(inserted.size_bytes, size_bytes);
        assert!(inserted.created_at > OffsetDateTime::UNIX_EPOCH);

        Ok(())
    }

    #[test]
    fn upsert_replaces_existing_image() -> Result<()> {
        let conn = conn();
        super::upsert("way", 1, vec![1, 2, 3], 600, 315, 3, &conn)?;
        super::upsert("way", 1, vec![4, 5], 300, 150, 2, &conn)?;

        let selected = super::select_by_osm_id("way", 1, &conn)?.unwrap();

        assert_eq!(selected.image_data, vec![4, 5]);
        assert_eq!(selected.width, 300);
        assert_eq!(selected.height, 150);
        assert_eq!(selected.size_bytes, 2);

        Ok(())
    }

    #[test]
    fn upsert_refreshes_created_at() -> Result<()> {
        let conn = conn();
        super::upsert("way", 1, vec![1], 600, 315, 1, &conn)?;
        conn.execute(
            &format!("UPDATE {TABLE} SET {CreatedAt} = '2000-01-01T00:00:00.000Z'"),
            [],
        )?;

        let refreshed = super::upsert("way", 1, vec![1], 600, 315, 1, &conn)?;

        assert!(refreshed.created_at > OffsetDateTime::now_utc() - Duration::days(1));

        Ok(())
    }

    #[test]
    fn select_by_osm_ref_matches_type_and_id() -> Result<()> {
        let conn = conn();
        super::upsert("node", 1, vec![1], 600, 315, 1, &conn)?;
        super::upsert("way", 1, vec![2], 600, 315, 1, &conn)?;

        assert_eq!(
            vec![1],
            super::select_by_osm_id("node", 1, &conn)?
                .unwrap()
                .image_data
        );
        assert_eq!(
            vec![2],
            super::select_by_osm_id("way", 1, &conn)?
                .unwrap()
                .image_data
        );
        assert!(super::select_by_osm_id("relation", 1, &conn)?.is_none());

        Ok(())
    }

    #[test]
    fn select_by_osm_ref_not_exists() -> Result<()> {
        let conn = conn();
        let res = super::select_by_osm_id("node", 9999, &conn)?;

        assert!(res.is_none());

        Ok(())
    }

    #[test]
    fn delete_created_before_keeps_fresh_rows() -> Result<()> {
        let conn = conn();
        super::upsert("node", 1, vec![1], 600, 315, 1, &conn)?;
        super::upsert("node", 2, vec![2], 600, 315, 1, &conn)?;
        conn.execute(
            &format!(
                "UPDATE {TABLE} SET {CreatedAt} = '2000-01-01T00:00:00.000Z' WHERE {OsmId} = 2"
            ),
            [],
        )?;

        let deleted =
            super::delete_created_before(OffsetDateTime::now_utc() - Duration::days(7), &conn)?;

        assert_eq!(1, deleted);
        assert!(super::select_by_osm_id("node", 1, &conn)?.is_some());
        assert!(super::select_by_osm_id("node", 2, &conn)?.is_none());

        Ok(())
    }
}
