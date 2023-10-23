use std::collections::HashMap;

use crate::Result;
use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Row;
use serde_json::Value;
use time::macros::format_description;
use time::Date;
use time::OffsetDateTime;

pub struct Report {
    pub id: i32,
    pub area_url_alias: String,
    pub date: Date,
    pub tags: HashMap<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Report {
    pub fn insert(
        area_url_alias: &str,
        date: &Date,
        tags: &HashMap<String, Value>,
        conn: &Connection,
    ) -> Result<()> {
        let query = r#"
            INSERT INTO report (
                area_url_alias,
                date,
                tags
            ) VALUES (
                :area_url_alias,
                :date,
                :tags
            )
        "#;

        conn.execute(
            query,
            named_params! {
                ":area_url_alias" : area_url_alias,
                ":date" : date.to_string(),
                ":tags" : serde_json::to_string(&tags)?,
            },
        )?;

        Ok(())
    }

    pub fn select_all(limit: Option<i32>, conn: &Connection) -> Result<Vec<Report>> {
        let query = r#"
            SELECT
                rowid,
                area_url_alias,
                date,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM report
            ORDER BY updated_at, rowid
            LIMIT :limit
        "#;

        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! { ":limit": limit.unwrap_or(std::i32::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<Report>, _>>()?)
    }

    pub fn select_updated_since(
        updated_since: &str,
        limit: Option<i32>,
        conn: &Connection,
    ) -> Result<Vec<Report>> {
        let query = r#"
        SELECT
            rowid,
            area_url_alias,
            date,
            tags,
            created_at,
            updated_at,
            deleted_at
        FROM report
        WHERE updated_at > :updated_since
        ORDER BY updated_at, rowid
        LIMIT :limit
        "#;

        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! { ":updated_since": updated_since, ":limit": limit.unwrap_or(std::i32::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<Report>, _>>()?)
    }

    pub fn select_by_id(id: i32, conn: &Connection) -> Result<Option<Report>> {
        let query = r#"
            SELECT
                rowid,
                area_url_alias,
                date,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM report
            WHERE rowid = :id
        "#;

        Ok(conn
            .query_row(query, named_params! { ":id": id }, mapper())
            .optional()?)
    }

    pub fn select_by_area_url_alias_and_date(
        area_url_alias: &str,
        date: &Date,
        conn: &Connection,
    ) -> Result<Option<Report>> {
        let query = r#"
            SELECT
                rowid,
                area_url_alias,
                date,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM report
            WHERE area_url_alias = :area_url_alias AND date = :date
        "#;

        Ok(conn
            .query_row(
                query,
                named_params! { ":area_url_alias": area_url_alias, ":date": date.to_string() },
                mapper(),
            )
            .optional()?)
    }

    pub fn merge_tags(id: i32, tags: &HashMap<String, Value>, conn: &Connection) -> Result<()> {
        let query = r#"
            UPDATE report
            SET tags = json_patch(tags, :tags)
            WHERE rowid = :id
        "#;

        conn.execute(
            query,
            named_params! { ":id": id, ":tags": &serde_json::to_string(tags)? },
        )?;

        Ok(())
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<Report> {
    |row: &Row| -> rusqlite::Result<Report> {
        let date: String = row.get(2)?;

        let tags: String = row.get(3)?;
        let tags: HashMap<String, Value> = serde_json::from_str(&tags).unwrap_or_default();

        Ok(Report {
            id: row.get(0)?,
            area_url_alias: row.get(1)?,
            date: Date::parse(&date, &format_description!("[year]-[month]-[day]")).unwrap(),
            tags: tags,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            deleted_at: row.get(6)?,
        })
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use time::OffsetDateTime;

    use crate::{command::db, Result};

    use super::Report;

    #[test]
    fn insert() -> Result<()> {
        let conn = db::setup_connection()?;
        Report::insert(
            "",
            &OffsetDateTime::now_utc().date(),
            &HashMap::new(),
            &conn,
        )?;
        let reports = Report::select_all(None, &conn)?;
        assert_eq!(1, reports.len());
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = db::setup_connection()?;
        Report::insert(
            "",
            &OffsetDateTime::now_utc().date(),
            &HashMap::new(),
            &conn,
        )?;
        Report::insert(
            "",
            &OffsetDateTime::now_utc().date(),
            &HashMap::new(),
            &conn,
        )?;
        Report::insert(
            "",
            &OffsetDateTime::now_utc().date(),
            &HashMap::new(),
            &conn,
        )?;
        let reports = Report::select_all(None, &conn)?;
        assert_eq!(3, reports.len());
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = db::setup_connection()?;
        conn.execute(
            "INSERT INTO report (area_url_alias, date, updated_at) VALUES ('', '2020-01-01', '2020-01-01T00:00:00Z')",
            [],
        )?;
        conn.execute(
            "INSERT INTO report (area_url_alias, date, updated_at) VALUES ('', '2020-01-02', '2020-01-02T00:00:00Z')",
            [],
        )?;
        conn.execute(
            "INSERT INTO report (area_url_alias, date, updated_at) VALUES ('', '2020-01-03', '2020-01-03T00:00:00Z')",
            [],
        )?;
        assert_eq!(
            2,
            Report::select_updated_since("2020-01-01T00:00:00Z", None, &conn,)?.len()
        );
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = db::setup_connection()?;
        Report::insert(
            "",
            &OffsetDateTime::now_utc().date(),
            &HashMap::new(),
            &conn,
        )?;
        assert!(Report::select_by_id(1, &conn)?.is_some());
        Ok(())
    }

    #[test]
    fn select_by_area_url_alias_and_date() -> Result<()> {
        let conn = db::setup_connection()?;
        Report::insert(
            "test",
            &OffsetDateTime::now_utc().date(),
            &HashMap::new(),
            &conn,
        )?;
        assert!(Report::select_by_area_url_alias_and_date(
            "test",
            &OffsetDateTime::now_utc().date(),
            &conn
        )?
        .is_some());
        Ok(())
    }

    #[test]
    fn merge_tags() -> Result<()> {
        let conn = db::setup_connection()?;
        let tag_1_name = "foo";
        let tag_1_value = "bar";
        let tag_2_name = "qwerty";
        let tag_2_value = "test";
        let mut tags = HashMap::new();
        tags.insert(tag_1_name.into(), tag_1_value.into());
        Report::insert(
            "",
            &OffsetDateTime::now_utc().date(),
            &HashMap::new(),
            &conn,
        )?;
        let event = Report::select_by_id(1, &conn)?.unwrap();
        assert!(event.tags.is_empty());
        Report::merge_tags(1, &tags, &conn)?;
        let event = Report::select_by_id(1, &conn)?.unwrap();
        assert_eq!(1, event.tags.len());
        tags.insert(tag_2_name.into(), tag_2_value.into());
        Report::merge_tags(1, &tags, &conn)?;
        let event = Report::select_by_id(1, &conn)?.unwrap();
        assert_eq!(2, event.tags.len());
        Ok(())
    }
}
