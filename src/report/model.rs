use crate::Error;
use crate::Result;
use deadpool_sqlite::Pool;
use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Row;
use serde_json::Map;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::Date;
use time::OffsetDateTime;

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

const TABLE: &str = "report";
const COL_ID: &str = "id";
const COL_AREA_ID: &str = "area_id";
const COL_DATE: &str = "date";
const COL_TAGS: &str = "tags";
const COL_CREATED_AT: &str = "created_at";
const COL_UPDATED_AT: &str = "updated_at";
const COL_DELETED_AT: &str = "deleted_at";

impl Report {
    pub async fn insert_async(
        area_id: i64,
        date: Date,
        tags: Map<String, Value>,
        pool: &Pool,
    ) -> Result<Report> {
        pool.get()
            .await?
            .interact(move |conn| Report::insert(area_id, &date, &tags, conn))
            .await?
    }

    pub fn insert(
        area_id: i64,
        date: &Date,
        tags: &Map<String, Value>,
        conn: &Connection,
    ) -> Result<Report> {
        let sql = format!(
            r#"
            INSERT INTO {TABLE} (
                {COL_AREA_ID},
                {COL_DATE},
                {COL_TAGS}
            ) VALUES (
                :area_id,
                :date,
                :tags
            )
            "#
        );
        conn.execute(
            &sql,
            named_params! {
                ":area_id" : area_id,
                ":date" : date.to_string(),
                ":tags" : serde_json::to_string(&tags)?,
            },
        )?;
        Report::select_by_id(conn.last_insert_rowid(), conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    pub fn select_updated_since(
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<Report>> {
        let sql = format!(
            r#"
            SELECT
                r.{COL_ID},
                r.{COL_AREA_ID},
                a.alias,
                r.{COL_DATE},
                r.{COL_TAGS},
                r.{COL_CREATED_AT},
                r.{COL_UPDATED_AT},
                r.{COL_DELETED_AT}
            FROM report r
            LEFT JOIN area a ON a.id = r.{COL_AREA_ID}
            WHERE r.{COL_UPDATED_AT} > :updated_since
            ORDER BY r.{COL_UPDATED_AT}, r.{COL_ID}
            LIMIT :limit
        "#
        );
        Ok(conn
            .prepare(&sql)?
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
        let sql = r#"
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
            .prepare(sql)?
            .query_map(
                named_params! {
                    ":date": date.to_string(),
                    ":limit": limit.unwrap_or(i64::MAX),
                },
                mapper(),
            )?
            .collect::<Result<Vec<Report>, _>>()?)
    }

    pub async fn select_by_area_id_async(
        area_id: i64,
        limit: Option<i64>,
        pool: &Pool,
    ) -> Result<Vec<Report>> {
        pool.get()
            .await?
            .interact(move |conn| Report::select_by_area_id(area_id, limit, conn))
            .await?
    }

    pub fn select_by_area_id(
        area_id: i64,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<Report>> {
        let sql = r#"
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
            ORDER BY r.updated_at, r.rowid
            LIMIT :limit
        "#;
        Ok(conn
            .prepare(sql)?
            .query_map(
                named_params! {
                    ":area_id": area_id,
                    ":limit": limit.unwrap_or(i64::MAX),
                },
                mapper(),
            )?
            .collect::<Result<Vec<Report>, _>>()?)
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<Report>> {
        let sql = r#"
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
        Ok(conn
            .query_row(sql, named_params! { ":id": id }, mapper())
            .optional()?)
    }

    pub fn select_latest_by_area_id(area_id: i64, conn: &Connection) -> Result<Option<Report>> {
        let sql = r#"
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
        Ok(conn
            .query_row(sql, named_params! { ":area_id": area_id }, mapper())
            .optional()?)
    }

    #[cfg(test)]
    pub fn patch_tags(id: i64, tags: &Map<String, Value>, conn: &Connection) -> Result<Report> {
        let sql = r#"
            UPDATE report
            SET tags = json_patch(tags, :tags)
            WHERE rowid = :id
        "#;
        conn.execute(
            sql,
            named_params! { ":id": id, ":tags": &serde_json::to_string(tags)? },
        )?;
        Report::select_by_id(id, conn)?.ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
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
        let sql = r#"
                UPDATE report
                SET updated_at = :updated_at
                WHERE id = :id
            "#
        .to_string();
        conn.execute(
            &sql,
            named_params! {
                ":id": id,
                ":updated_at": updated_at.format(&time::format_description::well_known::Rfc3339)?,
            },
        )?;
        Report::select_by_id(id, conn)?.ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    #[cfg(test)]
    pub fn set_deleted_at(
        id: i64,
        deleted_at: &OffsetDateTime,
        conn: &Connection,
    ) -> Result<Report> {
        let sql = r#"
                UPDATE report
                SET deleted_at = :deleted_at
                WHERE id = :id
            "#
        .to_string();
        conn.execute(
            &sql,
            named_params! {
                ":id": id,
                ":deleted_at": deleted_at.format(&time::format_description::well_known::Rfc3339)?,
            },
        )?;
        Report::select_by_id(id, conn)?.ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    pub fn total_elements(&self) -> i64 {
        self.tags
            .get("total_elements")
            .map(|it| it.as_i64().unwrap_or_default())
            .unwrap_or_default()
    }

    pub fn up_to_date_elements(&self) -> i64 {
        self.tags
            .get("up_to_date_elements")
            .map(|it| it.as_i64().unwrap_or_default())
            .unwrap_or_default()
    }

    pub fn days_since_verified(&self) -> i64 {
        let now = OffsetDateTime::now_utc();
        let now_str = now.format(&Rfc3339).unwrap();
        match self.tags.get("avg_verification_date") {
            Some(date) => {
                let date =
                    OffsetDateTime::parse(date.as_str().unwrap_or(&now_str), &Rfc3339).unwrap();
                (self.date - date.date()).whole_days()
            }
            None => 0,
        }
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
            tags,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            deleted_at: row.get(7)?,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::test::mock_db;
    use crate::{area::Area, report::Report, test::mock_conn, Result};
    use actix_web::test;
    use serde_json::Map;
    use serde_json::Value;
    use std::ops::Add;
    use time::macros::date;
    use time::{macros::datetime, Duration, OffsetDateTime};

    #[actix_web::test]
    async fn insert_async() -> Result<()> {
        let db = mock_db();
        let area = Area::insert(Area::mock_tags(), &db.conn)?;
        let date = date!(2023 - 01 - 01);
        let mut tags = Map::new();
        tags.insert("key".to_string(), Value::String("value".to_string()));
        let report = Report::insert_async(area.id, date.clone(), tags.clone(), &db.pool).await?;
        assert_eq!(report.area_id, area.id);
        assert_eq!(report.date, date);
        assert_eq!(report.tags, tags);
        Ok(())
    }

    #[test]
    async fn insert() -> Result<()> {
        let conn = mock_conn();
        Area::insert(Area::mock_tags(), &conn)?;
        Report::insert(1, &OffsetDateTime::now_utc().date(), &Map::new(), &conn)?;
        let reports = Report::select_updated_since(&datetime!(2000-01-01 00:00 UTC), None, &conn)?;
        assert_eq!(1, reports.len());
        Ok(())
    }

    #[test]
    async fn select_updated_since() -> Result<()> {
        let conn = mock_conn();
        Area::insert(Area::mock_tags(), &conn)?;
        let report_1 = Report::insert(1, &OffsetDateTime::now_utc().date(), &Map::new(), &conn)?;
        Report::_set_updated_at(report_1.id, &datetime!(2020-01-01 00:00:00 UTC), &conn)?;
        let report_2 = Report::insert(1, &OffsetDateTime::now_utc().date(), &Map::new(), &conn)?;
        Report::_set_updated_at(report_2.id, &datetime!(2020-01-02 00:00:00 UTC), &conn)?;
        let report_3 = Report::insert(1, &OffsetDateTime::now_utc().date(), &Map::new(), &conn)?;
        Report::_set_updated_at(report_3.id, &datetime!(2020-01-03 00:00:00 UTC), &conn)?;
        assert_eq!(
            2,
            Report::select_updated_since(&datetime!(2020-01-01 00:00 UTC), None, &conn)?.len(),
        );
        Ok(())
    }

    #[test]
    async fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        Area::insert(Area::mock_tags(), &conn)?;
        Report::insert(1, &OffsetDateTime::now_utc().date(), &Map::new(), &conn)?;
        assert!(Report::select_by_id(1, &conn)?.is_some());
        Ok(())
    }

    #[test]
    async fn select_latest_by_area_id() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(Area::mock_tags(), &conn)?;
        Report::insert(
            area.id,
            &OffsetDateTime::now_utc().date().previous_day().unwrap(),
            &Map::new(),
            &conn,
        )?;
        let latest_report = Report::insert(
            area.id,
            &OffsetDateTime::now_utc().date(),
            &Map::new(),
            &conn,
        )?;
        assert_eq!(
            latest_report,
            Report::select_latest_by_area_id(area.id, &conn)?.unwrap(),
        );
        Ok(())
    }

    #[test]
    async fn merge_tags() -> Result<()> {
        let conn = mock_conn();
        Area::insert(Area::mock_tags(), &conn)?;
        let tag_1_name = "foo";
        let tag_1_value = "bar";
        let tag_2_name = "qwerty";
        let tag_2_value = "test";
        let mut tags = Map::new();
        tags.insert(tag_1_name.into(), tag_1_value.into());
        Report::insert(1, &OffsetDateTime::now_utc().date(), &Map::new(), &conn)?;
        let report = Report::select_by_id(1, &conn)?.unwrap();
        assert!(report.tags.is_empty());
        Report::patch_tags(1, &tags, &conn)?;
        let report = Report::select_by_id(1, &conn)?.unwrap();
        assert_eq!(1, report.tags.len());
        tags.insert(tag_2_name.into(), tag_2_value.into());
        Report::patch_tags(1, &tags, &conn)?;
        let report = Report::select_by_id(1, &conn)?.unwrap();
        assert_eq!(2, report.tags.len());
        Ok(())
    }

    #[test]
    async fn set_deleted_at() -> Result<()> {
        let conn = mock_conn();
        Area::insert(Area::mock_tags(), &conn)?;
        let report = Report::insert(1, &OffsetDateTime::now_utc().date(), &Map::new(), &conn)?;
        let new_deleted_at = OffsetDateTime::now_utc().add(Duration::days(1));
        Report::set_deleted_at(report.id, &new_deleted_at, &conn)?;
        assert_eq!(
            new_deleted_at,
            Report::select_by_id(report.id, &conn)?
                .unwrap()
                .deleted_at
                .unwrap(),
        );
        Ok(())
    }
}
