use std::{collections::HashMap, sync::Arc};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{named_params, OptionalExtension, Row};
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::task::spawn_blocking;
use tracing::debug;

use crate::{model::Area, Error, Result};

const TABLE: &str = "area";
const ALL_COLUMNS: &str = "rowid, tags, created_at, updated_at, deleted_at";
const COL_ROWID: &str = "rowid";
const COL_TAGS: &str = "tags";
const _COL_CREATED_AT: &str = "created_at";
const COL_UPDATED_AT: &str = "updated_at";
const COL_DELETED_AT: &str = "deleted_at";

pub struct AreaRepo {
    pool: Arc<Pool<SqliteConnectionManager>>,
}

impl AreaRepo {
    pub fn new(pool: Arc<Pool<SqliteConnectionManager>>) -> AreaRepo {
        AreaRepo { pool }
    }

    pub async fn insert(&self, tags: &HashMap<String, Value>) -> Result<Area> {
        let query = format!(
            r#"
                INSERT INTO {TABLE} ({COL_TAGS}) 
                VALUES (json(:tags))
            "#
        );
        debug!(query);
        let pool = self.pool.clone();
        let tags = serde_json::to_string(tags)?;
        let id = spawn_blocking(move || -> Result<i64> {
            let conn = pool.clone().get()?;
            conn.execute(&query, named_params! { ":tags": tags })?;
            Ok(conn.last_insert_rowid())
        })
        .await??;
        Ok(self
            .select_by_id(id)
            .await?
            .ok_or(Error::DbTableRowNotFound)?)
    }

    pub async fn select_all(&self, limit: Option<i64>) -> Result<Vec<Area>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                ORDER BY {COL_UPDATED_AT}, {COL_ROWID}
                LIMIT :limit
            "#
        );
        debug!(query);
        let pool = self.pool.clone();
        spawn_blocking(move || {
            Ok(pool
                .clone()
                .get()?
                .prepare(&query)?
                .query_map(
                    named_params! { ":limit": limit.unwrap_or(i64::MAX) },
                    mapper(),
                )?
                .collect::<Result<Vec<_>, _>>()?)
        })
        .await?
    }

    pub async fn select_updated_since(
        &self,
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
    ) -> Result<Vec<Area>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_UPDATED_AT} > :updated_since
                ORDER BY {COL_UPDATED_AT}, {COL_ROWID}
                LIMIT :limit
            "#
        );
        debug!(query);
        let pool = self.pool.clone();
        let updated_since = updated_since.format(&Rfc3339)?;
        spawn_blocking(move || {
            Ok(pool
                .get()?
                .prepare(&query)?
                .query_map(
                    named_params! {
                        ":updated_since": updated_since,
                        ":limit": limit.unwrap_or(i64::MAX),
                    },
                    mapper(),
                )?
                .collect::<Result<Vec<_>, _>>()?)
        })
        .await?
    }

    pub async fn select_by_id(&self, id: i64) -> Result<Option<Area>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        let pool = self.pool.clone();
        spawn_blocking(move || {
            Ok(pool
                .get()?
                .query_row(&query, named_params! { ":id": id }, mapper())
                .optional()?)
        })
        .await?
    }

    pub async fn select_by_url_alias(&self, url_alias: &str) -> Result<Option<Area>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE json_extract({COL_TAGS}, '$.url_alias') = :url_alias
            "#
        );
        debug!(query);
        let pool = self.pool.clone();
        let url_alias = url_alias.to_string();
        spawn_blocking(move || {
            Ok(pool
                .get()?
                .query_row(&query, named_params! { ":url_alias": url_alias }, mapper())
                .optional()?)
        })
        .await?
    }

    pub async fn patch_tags(&self, id: i64, tags: &HashMap<String, Value>) -> Result<Area> {
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_TAGS} = json_patch({COL_TAGS}, :tags)
                WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        let pool = self.pool.clone();
        let tags = serde_json::to_string(tags)?;
        spawn_blocking(move || -> Result<()> {
            let conn = pool.get()?;
            conn.execute(
                &query,
                named_params! {
                    ":id": id,
                    ":tags": tags,
                },
            )?;
            Ok(())
        })
        .await??;
        Ok(self
            .select_by_id(id)
            .await?
            .ok_or(Error::DbTableRowNotFound)?)
    }

    pub async fn remove_tag(&self, id: i64, tag: &str) -> Result<Area> {
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_TAGS} = json_remove({COL_TAGS}, :tag)
                WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        let pool = self.pool.clone();
        let tag = tag.to_string();
        spawn_blocking(move || -> Result<()> {
            pool.get()?.execute(
                &query,
                named_params! {
                    ":id": id,
                    ":tag": format!("$.{tag}"),
                },
            )?;
            Ok(())
        })
        .await??;
        Ok(self
            .select_by_id(id)
            .await?
            .ok_or(Error::DbTableRowNotFound)?)
    }

    #[cfg(test)]
    pub async fn set_updated_at(&self, id: i64, updated_at: &OffsetDateTime) -> Result<Area> {
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_UPDATED_AT} = :updated_at
                WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        let pool = self.pool.clone();
        let updated_at = updated_at.format(&Rfc3339)?;
        spawn_blocking(move || -> Result<()> {
            pool.get()?.execute(
                &query,
                named_params! {
                    ":id": id,
                    ":updated_at": updated_at,
                },
            )?;
            Ok(())
        })
        .await??;
        Ok(self
            .select_by_id(id)
            .await?
            .ok_or(Error::DbTableRowNotFound)?)
    }

    pub async fn set_deleted_at(
        &self,
        id: i64,
        deleted_at: Option<OffsetDateTime>,
    ) -> Result<Area> {
        let pool = self.pool.clone();
        match deleted_at {
            Some(deleted_at) => {
                let query = format!(
                    r#"
                        UPDATE {TABLE}
                        SET {COL_DELETED_AT} = :deleted_at
                        WHERE {COL_ROWID} = :id
                    "#
                );
                debug!(query);
                let deleted_at = deleted_at.format(&Rfc3339)?;
                spawn_blocking(move || -> Result<()> {
                    pool.get()?.execute(
                        &query,
                        named_params! {
                            ":id": id,
                            ":deleted_at": deleted_at,
                        },
                    )?;
                    Ok(())
                })
                .await??;
            }
            None => {
                let query = format!(
                    r#"
                        UPDATE {TABLE}
                        SET {COL_DELETED_AT} = NULL
                        WHERE {COL_ROWID} = :id
                    "#
                );
                debug!(query);
                spawn_blocking(move || -> Result<()> {
                    pool.get()?.execute(&query, named_params! { ":id": id })?;
                    Ok(())
                })
                .await??;
            }
        };
        Ok(self
            .select_by_id(id)
            .await?
            .ok_or(Error::DbTableRowNotFound)?)
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<Area> {
    |row: &Row| -> rusqlite::Result<Area> {
        let tags: String = row.get(1)?;

        Ok(Area {
            id: row.get(0)?,
            tags: serde_json::from_str(&tags).unwrap(),
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
            deleted_at: row.get(4)?,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{
        test::{mock_area_repo, mock_tags},
        Result,
    };
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use time::{macros::datetime, OffsetDateTime};
    use tokio::test;

    #[test]
    async fn insert() -> Result<()> {
        let repo = mock_area_repo();
        let tags = mock_tags();
        let res = repo.insert(&tags).await?;
        assert_eq!(tags, res.tags);
        assert_eq!(res, repo.select_by_id(res.id).await?.unwrap());
        Ok(())
    }

    #[test]
    async fn select_all() -> Result<()> {
        let repo = mock_area_repo();
        assert_eq!(
            vec![
                repo.insert(&HashMap::new()).await?,
                repo.insert(&HashMap::new()).await?,
                repo.insert(&HashMap::new()).await?,
            ],
            repo.select_all(None).await?,
        );
        Ok(())
    }

    #[test]
    async fn select_updated_since() -> Result<()> {
        let repo = mock_area_repo();
        let _area_1 = repo.insert(&mock_tags()).await?;
        let _area_1 = repo
            .set_updated_at(_area_1.id, &datetime!(2020-01-01 00:00 UTC))
            .await?;
        let area_2 = repo.insert(&mock_tags()).await?;
        let area_2 = repo
            .set_updated_at(area_2.id, &datetime!(2020-01-02 00:00 UTC))
            .await?;
        let area_3 = repo.insert(&mock_tags()).await?;
        let area_3 = repo
            .set_updated_at(area_3.id, &datetime!(2020-01-03 00:00 UTC))
            .await?;
        assert_eq!(
            vec![area_2, area_3],
            repo.select_updated_since(&datetime!(2020-01-01 00:00 UTC), None)
                .await?,
        );
        Ok(())
    }

    #[test]
    async fn select_by_id() -> Result<()> {
        let repo = mock_area_repo();
        let area = repo.insert(&HashMap::new()).await?;
        assert_eq!(area, repo.select_by_id(area.id).await?.unwrap());
        Ok(())
    }

    #[test]
    async fn select_by_url_alias() -> Result<()> {
        let repo = mock_area_repo();
        let url_alias = json!("url_alias_value");
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), url_alias.clone());
        repo.insert(&tags).await?;
        let area = repo
            .select_by_url_alias(url_alias.as_str().unwrap())
            .await?;
        assert!(area.is_some());
        let area = area.unwrap();
        assert_eq!(url_alias, area.tags["url_alias"]);
        Ok(())
    }

    #[test]
    async fn patch_tags() -> Result<()> {
        let repo = mock_area_repo();
        let tag_1_name = "tag_1_name";
        let tag_1_value = json!("tag_1_value");
        let tag_2_name = "tag_2_name";
        let tag_2_value = json!("tag_2_value");
        let mut tags = HashMap::new();
        tags.insert(tag_1_name.into(), tag_1_value.clone());
        let area = repo.insert(&tags).await?;
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        tags.insert(tag_2_name.into(), tag_2_value.clone());
        let area = repo.patch_tags(area.id, &tags).await?;
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        assert_eq!(tag_2_value, area.tags[tag_2_name]);
        Ok(())
    }

    #[test]
    async fn remove_tag() -> Result<()> {
        let repo = mock_area_repo();
        let tag_name = "tag_name";
        let tag_value = json!("tag_value");
        let mut tags: HashMap<String, Value> = HashMap::new();
        tags.insert(tag_name.into(), tag_value.clone());
        let area = repo.insert(&tags).await?;
        assert_eq!(tag_value, area.tags[tag_name].as_str().unwrap());
        let area = repo.remove_tag(area.id, tag_name).await?;
        assert!(area.tags.get(tag_name).is_none());
        Ok(())
    }

    #[test]
    async fn set_deleted_at() -> Result<()> {
        let repo = mock_area_repo();
        let area = repo.insert(&HashMap::new()).await?;
        let area = repo
            .set_deleted_at(area.id, Some(OffsetDateTime::now_utc()))
            .await?;
        assert!(area.deleted_at.is_some());
        let area = repo.set_deleted_at(area.id, None).await?;
        assert!(area.deleted_at.is_none());
        Ok(())
    }
}
