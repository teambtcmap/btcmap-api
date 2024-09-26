use crate::Error;
use crate::Result;
use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Row;
use serde_json::Map;
use serde_json::Value;
#[cfg(not(test))]
use std::thread::sleep;
#[cfg(not(test))]
use std::time::Duration;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::Date;
use time::OffsetDateTime;
use tracing::debug;

#[derive(Debug, Eq, PartialEq)]
pub struct Report {
    pub id: i64,
    pub area_id: i64,
    pub area_url_alias: String,
    pub date: Date,
    pub tags: Map<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Report {
    pub fn insert(
        area_id: i64,
        date: &Date,
        tags: &Map<String, Value>,
        conn: &Connection,
    ) -> Result<Report> {
        let query = r#"
            INSERT INTO report (
                area_id,
                date,
                tags
            ) VALUES (
                :area_id,
                :date,
                :tags
            )
        "#;
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            query,
            named_params! {
                ":area_id" : area_id,
                ":date" : date.to_string(),
                ":tags" : serde_json::to_string(&tags)?,
            },
        )?;

        Ok(Report::select_by_id(conn.last_insert_rowid(), &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn select_updated_since(
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<Report>> {
        let query = r#"
            SELECT
                r.rowid,
                r.area_id,
                json_extract(a.tags, '$.url_alias'),
                r.date,
                r.tags,
                r.created_at,
                r.updated_at,
                r.deleted_at
            FROM report r
            LEFT JOIN area a ON a.rowid = r.area_id
            WHERE r.updated_at > :updated_since
            ORDER BY r.updated_at, r.rowid
            LIMIT :limit
        "#;

        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! {
                    ":updated_since": updated_since.format(&Rfc3339)?,
                    ":limit": limit.unwrap_or(i64::MAX),
                },
                mapper(),
            )?
            .collect::<Result<Vec<Report>, _>>()?)
    }

    pub fn select_by_date(
        date: &Date,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<Report>> {
        let query = r#"
            SELECT
                r.rowid,
                r.area_id,
                json_extract(a.tags, '$.url_alias'),
                r.date,
                r.tags,
                r.created_at,
                r.updated_at,
                r.deleted_at
            FROM report r
            LEFT JOIN area a ON a.rowid = r.area_id
            WHERE r.date = :date
            ORDER BY r.updated_at, r.rowid
            LIMIT :limit
        "#;

        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! {
                    ":date": date.to_string(),
                    ":limit": limit.unwrap_or(i64::MAX),
                },
                mapper(),
            )?
            .collect::<Result<Vec<Report>, _>>()?)
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<Report>> {
        let query = r#"
            SELECT
                r.rowid,
                r.area_id,
                json_extract(a.tags, '$.url_alias'),
                r.date,
                r.tags,
                r.created_at,
                r.updated_at,
                r.deleted_at
            FROM report r
            LEFT JOIN area a ON a.rowid = r.area_id
            WHERE r.rowid = :id
        "#;
        debug!(query);
        Ok(conn
            .query_row(query, named_params! { ":id": id }, mapper())
            .optional()?)
    }

    pub fn select_latest_by_area_id(area_id: i64, conn: &Connection) -> Result<Option<Report>> {
        let query = r#"
            SELECT
                r.rowid,
                r.area_id,
                json_extract(a.tags, '$.url_alias'),
                r.date,
                r.tags,
                r.created_at,
                r.updated_at,
                r.deleted_at
            FROM report r
            LEFT JOIN area a ON a.rowid = r.area_id
            WHERE r.area_id = :area_id
            ORDER BY r.created_at DESC, r.id DESC
            LIMIT 1
        "#;
        debug!(query);
        Ok(conn
            .query_row(query, named_params! { ":area_id": area_id }, mapper())
            .optional()?)
    }

    #[cfg(test)]
    pub fn patch_tags(id: i64, tags: &Map<String, Value>, conn: &Connection) -> Result<Report> {
        let query = r#"
            UPDATE report
            SET tags = json_patch(tags, :tags)
            WHERE rowid = :id
        "#;
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            query,
            named_params! { ":id": id, ":tags": &serde_json::to_string(tags)? },
        )?;
        Ok(Report::select_by_id(id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    #[cfg(test)]
    pub fn __set_updated_at(
        &self,
        updated_at: &OffsetDateTime,
        conn: &Connection,
    ) -> Result<Report> {
        Report::_set_updated_at(self.id, updated_at, conn)
    }

    #[cfg(test)]
    pub fn _set_updated_at(
        id: i64,
        updated_at: &OffsetDateTime,
        conn: &Connection,
    ) -> Result<Report> {
        let query = format!(
            r#"
                UPDATE report
                SET updated_at = :updated_at
                WHERE id = :id
            "#
        );
        debug!(query);
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            &query,
            named_params! {
                ":id": id,
                ":updated_at": updated_at.format(&time::format_description::well_known::Rfc3339)?,
            },
        )?;
        Ok(Report::select_by_id(id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    #[cfg(test)]
    pub fn set_deleted_at(
        id: i64,
        deleted_at: &OffsetDateTime,
        conn: &Connection,
    ) -> Result<Report> {
        let query = format!(
            r#"
                UPDATE report
                SET deleted_at = :deleted_at
                WHERE id = :id
            "#
        );
        debug!(query);
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            &query,
            named_params! {
                ":id": id,
                ":deleted_at": deleted_at.format(&time::format_description::well_known::Rfc3339)?,
            },
        )?;
        Ok(Report::select_by_id(id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<Report> {
    |row: &Row| -> rusqlite::Result<Report> {
        let date: String = row.get(3)?;

        let tags: String = row.get(4)?;
        let tags: Map<String, Value> = serde_json::from_str(&tags).unwrap_or_default();

        Ok(Report {
            id: row.get(0)?,
            area_id: row.get(1)?,
            area_url_alias: row.get(2)?,
            date: Date::parse(&date, &format_description!("[year]-[month]-[day]")).unwrap(),
            tags: tags,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            deleted_at: row.get(7)?,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{area::Area, report::Report, test::mock_state, Result};
    use geojson::{Feature, GeoJson};
    use serde_json::Map;
    use std::ops::Add;
    use time::{macros::datetime, Duration, OffsetDateTime};
    use tokio::test;

    #[test]
    async fn insert() -> Result<()> {
        let state = mock_state().await;
        let mut area_tags = Map::new();
        area_tags.insert("url_alias".into(), "test".into());
        Area::insert(GeoJson::Feature(Feature::default()), area_tags, &state.conn)?;
        Report::insert(
            1,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &state.conn,
        )?;
        let reports =
            Report::select_updated_since(&datetime!(2000-01-01 00:00 UTC), None, &state.conn)?;
        assert_eq!(1, reports.len());
        Ok(())
    }

    #[test]
    async fn select_updated_since() -> Result<()> {
        let state = mock_state().await;
        let mut area_tags = Map::new();
        area_tags.insert("url_alias".into(), "test".into());
        Area::insert(GeoJson::Feature(Feature::default()), area_tags, &state.conn)?;
        let report_1 = Report::insert(
            1,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &state.conn,
        )?;
        Report::_set_updated_at(
            report_1.id,
            &datetime!(2020-01-01 00:00:00 UTC),
            &state.conn,
        )?;
        let report_2 = Report::insert(
            1,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &state.conn,
        )?;
        Report::_set_updated_at(
            report_2.id,
            &datetime!(2020-01-02 00:00:00 UTC),
            &state.conn,
        )?;
        let report_3 = Report::insert(
            1,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &state.conn,
        )?;
        Report::_set_updated_at(
            report_3.id,
            &datetime!(2020-01-03 00:00:00 UTC),
            &state.conn,
        )?;
        assert_eq!(
            2,
            Report::select_updated_since(&datetime!(2020-01-01 00:00 UTC), None, &state.conn)?
                .len(),
        );
        Ok(())
    }

    #[test]
    async fn select_by_id() -> Result<()> {
        let state = mock_state().await;
        let mut area_tags = Map::new();
        area_tags.insert("url_alias".into(), "test".into());
        Area::insert(GeoJson::Feature(Feature::default()), area_tags, &state.conn)?;
        Report::insert(
            1,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &state.conn,
        )?;
        assert!(Report::select_by_id(1, &state.conn)?.is_some());
        Ok(())
    }

    #[test]
    async fn select_latest_by_area_id() -> Result<()> {
        let state = mock_state().await;
        let mut area_tags = Map::new();
        area_tags.insert("url_alias".into(), "test".into());
        let area = Area::insert(GeoJson::Feature(Feature::default()), area_tags, &state.conn)?;
        Report::insert(
            area.id,
            &OffsetDateTime::now_utc().date().previous_day().unwrap(),
            &Map::new(),
            &state.conn,
        )?;
        let latest_report = Report::insert(
            area.id,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &state.conn,
        )?;
        assert_eq!(
            latest_report,
            Report::select_latest_by_area_id(area.id, &state.conn)?.unwrap(),
        );
        Ok(())
    }

    #[test]
    async fn merge_tags() -> Result<()> {
        let state = mock_state().await;
        let mut area_tags = Map::new();
        area_tags.insert("url_alias".into(), "test".into());
        Area::insert(GeoJson::Feature(Feature::default()), area_tags, &state.conn)?;
        let tag_1_name = "foo";
        let tag_1_value = "bar";
        let tag_2_name = "qwerty";
        let tag_2_value = "test";
        let mut tags = Map::new();
        tags.insert(tag_1_name.into(), tag_1_value.into());
        Report::insert(
            1,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &state.conn,
        )?;
        let report = Report::select_by_id(1, &state.conn)?.unwrap();
        assert!(report.tags.is_empty());
        Report::patch_tags(1, &tags, &state.conn)?;
        let report = Report::select_by_id(1, &state.conn)?.unwrap();
        assert_eq!(1, report.tags.len());
        tags.insert(tag_2_name.into(), tag_2_value.into());
        Report::patch_tags(1, &tags, &state.conn)?;
        let report = Report::select_by_id(1, &state.conn)?.unwrap();
        assert_eq!(2, report.tags.len());
        Ok(())
    }

    #[test]
    async fn set_deleted_at() -> Result<()> {
        let state = mock_state().await;
        let mut area_tags = Map::new();
        area_tags.insert("url_alias".into(), "test".into());
        Area::insert(GeoJson::Feature(Feature::default()), area_tags, &state.conn)?;
        let report = Report::insert(
            1,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &state.conn,
        )?;
        let new_deleted_at = OffsetDateTime::now_utc().add(Duration::days(1));
        Report::set_deleted_at(report.id, &new_deleted_at, &state.conn)?;
        assert_eq!(
            new_deleted_at,
            Report::select_by_id(report.id, &state.conn)?
                .unwrap()
                .deleted_at
                .unwrap(),
        );
        Ok(())
    }
}
