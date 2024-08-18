use crate::{Error, Result};
use geojson::{GeoJson, Geometry};
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use serde_json::{Map, Value};
use std::{
    thread::sleep,
    time::{Duration, Instant},
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tracing::{debug, error, info};

#[derive(Debug, Eq, PartialEq)]
pub struct Area {
    pub id: i64,
    pub tags: Map<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

const TABLE: &str = "area";
const ALL_COLUMNS: &str = "id, tags, created_at, updated_at, deleted_at";
const COL_ID: &str = "id";
const COL_TAGS: &str = "tags";
const _COL_CREATED_AT: &str = "created_at";
const COL_UPDATED_AT: &str = "updated_at";
const COL_DELETED_AT: &str = "deleted_at";

impl Area {
    pub fn insert(tags: Map<String, Value>, conn: &Connection) -> Result<Area> {
        sleep(Duration::from_millis(10));
        let query = format!(
            r#"
                INSERT INTO {TABLE} ({COL_TAGS}) 
                VALUES (json(:tags))
            "#
        );
        debug!(query);
        conn.execute(&query, named_params! { ":tags": Value::from(tags) })?;
        Ok(Area::select_by_id(conn.last_insert_rowid(), conn)?.unwrap())
    }

    pub fn select_all(limit: Option<i64>, conn: &Connection) -> Result<Vec<Area>> {
        let start = Instant::now();
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
                LIMIT :limit
            "#
        );
        debug!(query);
        let res = conn
            .prepare(&query)?
            .query_map(
                named_params! { ":limit": limit.unwrap_or(i64::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let time_ms = start.elapsed().as_millis();
        info!(
            count = res.len(),
            time_ms,
            "Loaded {} areas in {} ms",
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
        let start = Instant::now();
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_UPDATED_AT} > :updated_since
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
                LIMIT :limit
            "#
        );
        debug!(query);
        let res = conn
            .prepare(&query)?
            .query_map(
                named_params! {
                    ":updated_since": updated_since.format(&Rfc3339)?,
                    ":limit": limit.unwrap_or(i64::MAX),
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let time_ms = start.elapsed().as_millis();
        info!(
            count = res.len(),
            time_ms,
            "Loaded {} areas in {} ms",
            res.len(),
            time_ms,
        );
        Ok(res)
    }

    pub fn select_by_id_or_alias(id_or_alias: &str, conn: &Connection) -> Result<Option<Area>> {
        match id_or_alias.parse::<i64>() {
            Ok(id) => Area::select_by_id(id, conn),
            Err(_) => Area::select_by_alias(id_or_alias, conn),
        }
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<Area>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE {COL_ID} = :id
            "#
        );
        debug!(query);
        Ok(conn
            .query_row(&query, named_params! { ":id": id }, mapper())
            .optional()?)
    }

    pub fn select_by_alias(alias: &str, conn: &Connection) -> Result<Option<Area>> {
        let query = format!(
            r#"
                SELECT {ALL_COLUMNS}
                FROM {TABLE}
                WHERE json_extract({COL_TAGS}, '$.url_alias') = :alias
            "#
        );
        debug!(query);
        Ok(conn
            .query_row(&query, named_params! { ":alias": alias }, mapper())
            .optional()?)
    }

    pub fn patch_tags(id: i64, tags: Map<String, Value>, conn: &Connection) -> Result<Area> {
        sleep(Duration::from_millis(10));
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_TAGS} = json_patch({COL_TAGS}, json(:tags))
                WHERE {COL_ID} = :id
            "#
        );
        debug!(query);
        conn.execute(
            &query,
            named_params! {
                ":id": id,
                ":tags": Value::from(tags),
            },
        )?;
        Ok(Area::select_by_id(id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn remove_tag(id: i64, name: &str, conn: &Connection) -> Result<Area> {
        sleep(Duration::from_millis(10));
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_TAGS} = json_remove({COL_TAGS}, :name)
                WHERE {COL_ID} = :id
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
    pub fn set_updated_at(id: i64, updated_at: &OffsetDateTime, conn: &Connection) -> Result<Area> {
        sleep(Duration::from_millis(10));
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_UPDATED_AT} = :updated_at
                WHERE {COL_ID} = :id
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
        id: i64,
        deleted_at: Option<OffsetDateTime>,
        conn: &Connection,
    ) -> Result<Area> {
        sleep(Duration::from_millis(10));
        match deleted_at {
            Some(deleted_at) => {
                let query = format!(
                    r#"
                        UPDATE {TABLE}
                        SET {COL_DELETED_AT} = :deleted_at
                        WHERE {COL_ID} = :id
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
                        WHERE {COL_ID} = :id
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
        let res = Area::insert(tags.clone(), &state.conn)?;
        assert_eq!(tags, res.tags);
        assert_eq!(res, Area::select_by_id(res.id, &state.conn)?.unwrap());
        Ok(())
    }

    #[test]
    async fn select_all() -> Result<()> {
        let conn = mock_conn();
        assert_eq!(
            vec![
                Area::insert(Map::new(), &conn)?,
                Area::insert(Map::new(), &conn)?,
                Area::insert(Map::new(), &conn)?,
            ],
            Area::select_all(None, &conn)?,
        );
        Ok(())
    }

    #[test]
    async fn select_updated_since() -> Result<()> {
        let state = mock_state().await;
        let _area_1 = Area::insert(mock_tags(), &state.conn)?;
        let _area_1 =
            Area::set_updated_at(_area_1.id, &datetime!(2020-01-01 00:00 UTC), &state.conn)?;
        let area_2 = Area::insert(mock_tags(), &state.conn)?;
        let area_2 =
            Area::set_updated_at(area_2.id, &datetime!(2020-01-02 00:00 UTC), &state.conn)?;
        let area_3 = Area::insert(mock_tags(), &state.conn)?;
        let area_3 =
            Area::set_updated_at(area_3.id, &datetime!(2020-01-03 00:00 UTC), &state.conn)?;
        assert_eq!(
            vec![area_2, area_3],
            Area::select_updated_since(&datetime!(2020-01-01 00:00 UTC), None, &state.conn)?,
        );
        Ok(())
    }

    #[test]
    async fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(Map::new(), &conn)?;
        assert_eq!(area, Area::select_by_id(area.id, &conn)?.unwrap());
        Ok(())
    }

    #[test]
    async fn select_by_url_alias() -> Result<()> {
        let conn = mock_conn();
        let url_alias = json!("url_alias_value");
        let mut tags = Map::new();
        tags.insert("url_alias".into(), url_alias.clone());
        Area::insert(tags, &conn)?;
        let area = Area::select_by_alias(url_alias.as_str().unwrap(), &conn)?;
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
        let area = Area::insert(tags.clone(), &conn)?;
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        tags.insert(tag_2_name.into(), tag_2_value.clone());
        let area = Area::patch_tags(area.id, tags, &conn)?;
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        assert_eq!(tag_2_value, area.tags[tag_2_name]);
        Ok(())
    }

    #[test]
    async fn set_deleted_at() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(Map::new(), &conn)?;
        let area = Area::set_deleted_at(area.id, Some(OffsetDateTime::now_utc()), &conn)?;
        assert!(area.deleted_at.is_some());
        let area = Area::set_deleted_at(area.id, None, &conn)?;
        assert!(area.deleted_at.is_none());
        Ok(())
    }
}
