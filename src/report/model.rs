use crate::Error;
use crate::Result;
use deadpool_sqlite::Pool;
use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Row;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use time::macros::format_description;
use time::Date;
use time::OffsetDateTime;
use tracing::debug;

pub struct ReportRepo {
    pool: Arc<Pool>,
}

pub struct Report {
    pub id: i64,
    pub area_id: i64,
    pub area_url_alias: String,
    pub date: Date,
    pub tags: HashMap<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl ReportRepo {
    pub fn new(pool: &Arc<Pool>) -> Self {
        Self { pool: pool.clone() }
    }

    #[cfg(test)]
    pub async fn insert(
        &self,
        area_id: i64,
        date: &Date,
        tags: &HashMap<String, Value>,
    ) -> Result<Report> {
        let date = date.clone();
        let tags = tags.clone();
        self.pool
            .get()
            .await?
            .interact(move |conn| Report::insert(area_id, &date, &tags, conn))
            .await?
    }

    #[cfg(test)]
    pub async fn _select_all(&self, limit: Option<i64>) -> Result<Vec<Report>> {
        self.pool
            .get()
            .await?
            .interact(move |conn| Report::_select_all(limit, conn))
            .await?
    }

    pub async fn select_updated_since(
        &self,
        updated_since: &str,
        limit: Option<i64>,
    ) -> Result<Vec<Report>> {
        let updated_since = updated_since.to_string();
        self.pool
            .get()
            .await?
            .interact(move |conn| Report::select_updated_since(&updated_since, limit, conn))
            .await?
    }

    pub async fn select_by_id(&self, id: i64) -> Result<Option<Report>> {
        self.pool
            .get()
            .await?
            .interact(move |conn| Report::select_by_id(id, conn))
            .await?
    }

    #[cfg(test)]
    pub async fn patch_tags(&self, id: i64, tags: &HashMap<String, Value>) -> Result<Report> {
        let tags = tags.clone();
        self.pool
            .get()
            .await?
            .interact(move |conn| Report::patch_tags(id, &tags, conn))
            .await?
    }

    #[cfg(test)]
    pub async fn set_updated_at(&self, id: i64, updated_at: &OffsetDateTime) -> Result<Report> {
        let updated_at = updated_at.clone();
        self.pool
            .get()
            .await?
            .interact(move |conn| Report::_set_updated_at(id, &updated_at, conn))
            .await?
    }
}

impl Report {
    pub fn insert(
        area_id: i64,
        date: &Date,
        tags: &HashMap<String, Value>,
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

    #[cfg(test)]
    pub fn _select_all(limit: Option<i64>, conn: &Connection) -> Result<Vec<Report>> {
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
            ORDER BY r.updated_at, r.rowid
            LIMIT :limit
        "#;

        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! { ":limit": limit.unwrap_or(i64::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<Report>, _>>()?)
    }

    pub fn select_updated_since(
        updated_since: &str,
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
                named_params! { ":updated_since": updated_since, ":limit": limit.unwrap_or(i64::MAX) },
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

    #[cfg(test)]
    pub fn patch_tags(id: i64, tags: &HashMap<String, Value>, conn: &Connection) -> Result<Report> {
        let query = r#"
            UPDATE report
            SET tags = json_patch(tags, :tags)
            WHERE rowid = :id
        "#;
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
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<Report> {
    |row: &Row| -> rusqlite::Result<Report> {
        let date: String = row.get(3)?;

        let tags: String = row.get(4)?;
        let tags: HashMap<String, Value> = serde_json::from_str(&tags).unwrap_or_default();

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
    use crate::{test::mock_state, Result};
    use std::collections::HashMap;
    use time::{macros::datetime, OffsetDateTime};
    use tokio::test;

    #[test]
    async fn insert() -> Result<()> {
        let state = mock_state().await;
        let mut area_tags = HashMap::new();
        area_tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&area_tags).await?;
        state
            .report_repo
            .insert(1, &OffsetDateTime::now_utc().date(), &HashMap::new())
            .await?;
        let reports = state
            .report_repo
            .select_updated_since("2000-01-01", None)
            .await?;
        assert_eq!(1, reports.len());
        Ok(())
    }

    #[test]
    async fn select_all() -> Result<()> {
        let state = mock_state().await;
        let mut area_tags = HashMap::new();
        area_tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&area_tags).await?;
        state
            .report_repo
            .insert(1, &OffsetDateTime::now_utc().date(), &HashMap::new())
            .await?;
        state
            .report_repo
            .insert(1, &OffsetDateTime::now_utc().date(), &HashMap::new())
            .await?;
        state
            .report_repo
            .insert(1, &OffsetDateTime::now_utc().date(), &HashMap::new())
            .await?;
        let reports = state
            .report_repo
            .select_updated_since("2000-01-01", None)
            .await?;
        assert_eq!(3, reports.len());
        Ok(())
    }

    #[test]
    async fn select_updated_since() -> Result<()> {
        let state = mock_state().await;
        let mut area_tags = HashMap::new();
        area_tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&area_tags).await?;
        let report_1 = state
            .report_repo
            .insert(1, &OffsetDateTime::now_utc().date(), &HashMap::new())
            .await?;
        state
            .report_repo
            .set_updated_at(report_1.id, &datetime!(2020-01-01 00:00:00 UTC))
            .await?;
        let report_2 = state
            .report_repo
            .insert(1, &OffsetDateTime::now_utc().date(), &HashMap::new())
            .await?;
        state
            .report_repo
            .set_updated_at(report_2.id, &datetime!(2020-01-02 00:00:00 UTC))
            .await?;
        let report_3 = state
            .report_repo
            .insert(1, &OffsetDateTime::now_utc().date(), &HashMap::new())
            .await?;
        state
            .report_repo
            .set_updated_at(report_3.id, &datetime!(2020-01-03 00:00:00 UTC))
            .await?;
        assert_eq!(
            2,
            state
                .report_repo
                .select_updated_since("2020-01-01T00:00:00Z", None)
                .await?
                .len()
        );
        Ok(())
    }

    #[test]
    async fn select_by_id() -> Result<()> {
        let state = mock_state().await;
        let mut area_tags = HashMap::new();
        area_tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&area_tags).await?;
        state
            .report_repo
            .insert(1, &OffsetDateTime::now_utc().date(), &HashMap::new())
            .await?;
        assert!(state.report_repo.select_by_id(1).await?.is_some());
        Ok(())
    }

    #[test]
    async fn merge_tags() -> Result<()> {
        let state = mock_state().await;
        let mut area_tags = HashMap::new();
        area_tags.insert("url_alias".into(), "test".into());
        state.area_repo.insert(&area_tags).await?;
        let tag_1_name = "foo";
        let tag_1_value = "bar";
        let tag_2_name = "qwerty";
        let tag_2_value = "test";
        let mut tags = HashMap::new();
        tags.insert(tag_1_name.into(), tag_1_value.into());
        state
            .report_repo
            .insert(1, &OffsetDateTime::now_utc().date(), &HashMap::new())
            .await?;
        let report = state.report_repo.select_by_id(1).await?.unwrap();
        assert!(report.tags.is_empty());
        state.report_repo.patch_tags(1, &tags).await?;
        let report = state.report_repo.select_by_id(1).await?.unwrap();
        assert_eq!(1, report.tags.len());
        tags.insert(tag_2_name.into(), tag_2_value.into());
        state.report_repo.patch_tags(1, &tags).await?;
        let report = state.report_repo.select_by_id(1).await?.unwrap();
        assert_eq!(2, report.tags.len());
        Ok(())
    }
}
