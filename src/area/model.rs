use crate::{Error, Result};
use deadpool_sqlite::Pool;
use geojson::{GeoJson, Geometry};
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use serde_json::{Map, Value};
use std::{sync::Arc, time::Instant};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tracing::{debug, error, info};

#[derive(Clone)]
pub struct AreaRepo {
    pool: Arc<Pool>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Area {
    pub id: i64,
    pub tags: Map<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl AreaRepo {
    pub fn new(pool: &Arc<Pool>) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn select_all(&self, limit: Option<i64>) -> Result<Vec<Area>> {
        self.pool
            .get()
            .await?
            .interact(move |conn| Area::select_all(limit, conn))
            .await?
    }

    pub async fn select_updated_since(
        &self,
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
    ) -> Result<Vec<Area>> {
        let updated_since = updated_since.clone();
        self.pool
            .get()
            .await?
            .interact(move |conn| Area::select_updated_since(&updated_since, limit, conn))
            .await?
    }

    #[cfg(test)]
    pub async fn select_by_id(&self, id: i64) -> Result<Option<Area>> {
        self.pool
            .get()
            .await?
            .interact(move |conn| Area::select_by_id(id, conn))
            .await?
    }

    pub async fn select_by_url_alias(&self, url_alias: &str) -> Result<Option<Area>> {
        let url_alias = url_alias.to_string();
        self.pool
            .get()
            .await?
            .interact(move |conn| Area::select_by_url_alias(&url_alias, conn))
            .await?
    }

    #[cfg(test)]
    pub async fn set_updated_at(&self, id: i64, updated_at: &OffsetDateTime) -> Result<Area> {
        let updated_at = updated_at.clone();
        self.pool
            .get()
            .await?
            .interact(move |conn| Area::_set_updated_at(id, &updated_at, conn))
            .await?
    }
}

const TABLE: &str = "area";
const ALL_COLUMNS: &str = "rowid, tags, created_at, updated_at, deleted_at";
const COL_ROWID: &str = "rowid";
const COL_TAGS: &str = "tags";
const _COL_CREATED_AT: &str = "created_at";
const COL_UPDATED_AT: &str = "updated_at";
const COL_DELETED_AT: &str = "deleted_at";

impl Area {
    pub fn insert(tags: &Map<String, Value>, conn: &Connection) -> Result<Area> {
        let query = format!(
            r#"
                INSERT INTO {TABLE} ({COL_TAGS}) 
                VALUES (json(:tags))
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! { ":tags": serde_json::to_string(tags)? },
        )?;
        Ok(Area::select_by_id(conn.last_insert_rowid(), conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn select_all(limit: Option<i64>, conn: &Connection) -> Result<Vec<Area>> {
        let start = Instant::now();
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                ORDER BY {COL_UPDATED_AT}, {COL_ROWID}
                LIMIT :limit
            "#
        );
        debug!(query);
        let res = conn
            .prepare(&query)?
            .query_map(
                named_params! { ":limit": limit.unwrap_or(i64::MAX) },
                Self::mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let time_ms = start.elapsed().as_millis();
        info!(
            count = res.len(),
            time_ms,
            "Loaded all areas ({}) in {} ms",
            res.len(),
            time_ms,
        );
        Ok(res)
    }

    pub fn select_updated_since(
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
        conn: &Connection,
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
        Ok(conn
            .prepare(&query)?
            .query_map(
                named_params! {
                    ":updated_since": updated_since.format(&Rfc3339)?,
                    ":limit": limit.unwrap_or(i64::MAX),
                },
                Self::mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub async fn select_by_id_or_alias_async(
        id_or_alias: &str,
        pool: &Pool,
    ) -> Result<Option<Area>> {
        let id_or_alias = id_or_alias.to_string();
        pool.get()
            .await?
            .interact(move |conn| Area::select_by_id_or_alias(&id_or_alias, conn))
            .await?
    }

    pub fn select_by_id_or_alias(id_or_alias: &str, conn: &Connection) -> Result<Option<Area>> {
        match id_or_alias.parse::<i64>() {
            Ok(id) => Area::select_by_id(id, conn),
            Err(_) => Area::select_by_url_alias(id_or_alias, conn),
        }
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<Area>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        Ok(conn
            .query_row(&query, named_params! { ":id": id }, Self::mapper())
            .optional()?)
    }

    pub fn select_by_url_alias(url_alias: &str, conn: &Connection) -> Result<Option<Area>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE json_extract({COL_TAGS}, '$.url_alias') = :url_alias
            "#
        );
        debug!(query);
        Ok(conn
            .query_row(
                &query,
                named_params! { ":url_alias": url_alias },
                Self::mapper(),
            )
            .optional()?)
    }

    pub fn patch_tags(&self, tags: &Map<String, Value>, conn: &Connection) -> Result<Area> {
        Area::_patch_tags(self.id, tags, conn)
    }

    pub fn _patch_tags(id: i64, tags: &Map<String, Value>, conn: &Connection) -> Result<Area> {
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_TAGS} = json_patch({COL_TAGS}, :tags)
                WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! {
                ":id": id,
                ":tags": serde_json::to_string(tags)?,
            },
        )?;
        Ok(Area::select_by_id(id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn remove_tag(&self, name: &str, conn: &Connection) -> Result<Area> {
        Area::_remove_tag(self.id, name, conn)
    }

    fn _remove_tag(id: i64, name: &str, conn: &Connection) -> Result<Area> {
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_TAGS} = json_remove({COL_TAGS}, :name)
                WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! {
                ":id": id,
                ":name": format!("$.{name}"),
            },
        )?;
        Ok(Area::select_by_id(id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    #[cfg(test)]
    pub fn __set_updated_at(&self, updated_at: &OffsetDateTime, conn: &Connection) -> Result<Area> {
        Area::_set_updated_at(self.id, updated_at, conn)
    }

    #[cfg(test)]
    pub fn _set_updated_at(
        id: i64,
        updated_at: &OffsetDateTime,
        conn: &Connection,
    ) -> Result<Area> {
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_UPDATED_AT} = :updated_at
                WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! {
                ":id": id,
                ":updated_at": updated_at.format(&Rfc3339)?,
            },
        )?;
        Ok(Area::select_by_id(id, conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn set_deleted_at(
        &self,
        deleted_at: Option<OffsetDateTime>,
        conn: &Connection,
    ) -> Result<Area> {
        Area::_set_deleted_at(self.id, deleted_at, conn)
    }

    pub fn _set_deleted_at(
        id: i64,
        deleted_at: Option<OffsetDateTime>,
        conn: &Connection,
    ) -> Result<Area> {
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
                conn.execute(
                    &query,
                    named_params! {
                        ":id": id,
                        ":deleted_at": deleted_at.format(&Rfc3339)?,
                    },
                )?;
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
                conn.execute(&query, named_params! { ":id": id })?;
            }
        };
        Ok(Area::select_by_id(id, conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn name(&self) -> String {
        self.tags
            .get("name")
            .unwrap_or(&Value::String("".into()))
            .as_str()
            .unwrap()
            .into()
    }

    pub fn alias(&self) -> String {
        self.tags
            .get("url_alias")
            .unwrap_or(&Value::String("".into()))
            .as_str()
            .unwrap()
            .into()
    }

    pub fn geo_json(&self) -> Option<GeoJson> {
        let geo_json = self.tags.get("geo_json").unwrap_or(&Value::Null);

        if !geo_json.is_object() {
            error!(self.id, "geo_json is missing or not an object");
            return None;
        }

        let geo_json: Result<GeoJson, _> = serde_json::to_string(geo_json).unwrap().parse();

        match geo_json {
            Ok(geo_json) => Some(geo_json),
            Err(e) => {
                error!(self.id, error = e.to_string(), "Failed to parse geo_json");
                None
            }
        }
    }

    pub fn geo_json_geometries(&self) -> Vec<Geometry> {
        let geo_json = match self.geo_json() {
            Some(geo_json) => geo_json,
            None => return vec![],
        };

        let mut geometries: Vec<Geometry> = vec![];

        match geo_json {
            GeoJson::FeatureCollection(v) => {
                for feature in v.features {
                    if let Some(v) = feature.geometry {
                        geometries.push(v);
                    }
                }
            }
            GeoJson::Feature(v) => {
                if let Some(v) = v.geometry {
                    geometries.push(v);
                }
            }
            GeoJson::Geometry(v) => geometries.push(v),
        };

        return geometries;
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
}

#[cfg(test)]
mod test {
    use crate::{
        area::Area,
        test::{mock_conn, mock_state, mock_tags},
        Result,
    };
    use serde_json::{json, Map};
    use time::{macros::datetime, OffsetDateTime};
    use tokio::test;

    #[test]
    async fn insert() -> Result<()> {
        let state = mock_state().await;
        let tags = mock_tags();
        let res = Area::insert(&tags, &state.conn)?;
        assert_eq!(tags, res.tags);
        assert_eq!(res, Area::select_by_id(res.id, &state.conn)?.unwrap());
        Ok(())
    }

    #[test]
    async fn select_all() -> Result<()> {
        let state = mock_state().await;
        assert_eq!(
            vec![
                Area::insert(&Map::new(), &state.conn)?,
                Area::insert(&Map::new(), &state.conn)?,
                Area::insert(&Map::new(), &state.conn)?,
            ],
            state.area_repo.select_all(None).await?,
        );
        Ok(())
    }

    #[test]
    async fn select_updated_since() -> Result<()> {
        let state = mock_state().await;
        let _area_1 = Area::insert(&mock_tags(), &state.conn)?;
        let _area_1 = state
            .area_repo
            .set_updated_at(_area_1.id, &datetime!(2020-01-01 00:00 UTC))
            .await?;
        let area_2 = Area::insert(&mock_tags(), &state.conn)?;
        let area_2 = state
            .area_repo
            .set_updated_at(area_2.id, &datetime!(2020-01-02 00:00 UTC))
            .await?;
        let area_3 = Area::insert(&mock_tags(), &state.conn)?;
        let area_3 = state
            .area_repo
            .set_updated_at(area_3.id, &datetime!(2020-01-03 00:00 UTC))
            .await?;
        assert_eq!(
            vec![area_2, area_3],
            state
                .area_repo
                .select_updated_since(&datetime!(2020-01-01 00:00 UTC), None)
                .await?,
        );
        Ok(())
    }

    #[test]
    async fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(&Map::new(), &conn)?;
        assert_eq!(area, Area::select_by_id(area.id, &conn)?.unwrap());
        Ok(())
    }

    #[test]
    async fn select_by_url_alias() -> Result<()> {
        let conn = mock_conn();
        let url_alias = json!("url_alias_value");
        let mut tags = Map::new();
        tags.insert("url_alias".into(), url_alias.clone());
        Area::insert(&tags, &conn)?;
        let area = Area::select_by_url_alias(url_alias.as_str().unwrap(), &conn)?;
        assert!(area.is_some());
        let area = area.unwrap();
        assert_eq!(url_alias, area.tags["url_alias"]);
        Ok(())
    }

    #[test]
    async fn patch_tags() -> Result<()> {
        let conn = mock_conn();
        let tag_1_name = "tag_1_name";
        let tag_1_value = json!("tag_1_value");
        let tag_2_name = "tag_2_name";
        let tag_2_value = json!("tag_2_value");
        let mut tags = Map::new();
        tags.insert(tag_1_name.into(), tag_1_value.clone());
        let area = Area::insert(&tags, &conn)?;
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        tags.insert(tag_2_name.into(), tag_2_value.clone());
        let area = Area::_patch_tags(area.id, &tags, &conn)?;
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        assert_eq!(tag_2_value, area.tags[tag_2_name]);
        Ok(())
    }

    #[test]
    async fn set_deleted_at() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(&Map::new(), &conn)?;
        let area = Area::_set_deleted_at(area.id, Some(OffsetDateTime::now_utc()), &conn)?;
        assert!(area.deleted_at.is_some());
        let area = Area::_set_deleted_at(area.id, None, &conn)?;
        assert!(area.deleted_at.is_none());
        Ok(())
    }
}
