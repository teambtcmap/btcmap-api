use crate::Result;
use geojson::{GeoJson, Geometry};
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use serde_json::{Map, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Area {
    pub id: i64,
    pub tags: Map<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

const TABLE_NAME: &str = "area";
const COL_ID: &str = "id";
const COL_TAGS: &str = "tags";
const COL_ALIAS: &str = "alias";
const COL_UPDATED_AT: &str = "updated_at";
const COL_DELETED_AT: &str = "deleted_at";
const MAPPER_PROJECTION: &str = "id, tags, created_at, updated_at, deleted_at";

impl Area {
    pub fn insert(tags: Map<String, Value>, conn: &Connection) -> Result<Option<Area>> {
        let alias = tags
            .get("url_alias")
            .cloned()
            .ok_or("url_alias is missing")?;
        let alias = alias.as_str().ok_or("url_alias is not a string")?;
        let _ = tags.get("geo_json").ok_or("geo_json is missing")?;
        let geo_json = tags["geo_json"].clone();
        serde_json::to_string(&geo_json)?.parse::<GeoJson>()?;
        let sql = format!(
            r#"
                INSERT INTO {TABLE_NAME} ({COL_TAGS}, {COL_ALIAS})
                VALUES (json(:{COL_TAGS}), :{COL_ALIAS})
            "#
        );
        conn.execute(
            &sql,
            named_params! { ":tags": Value::from(tags), ":alias": alias },
        )?;
        Area::select_by_id(conn.last_insert_rowid(), conn)
    }

    pub fn select_all(conn: &Connection) -> Result<Vec<Area>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
            "#
        );
        conn.prepare(&sql)?
            .query_map({}, mapper())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn select_all_except_deleted(conn: &Connection) -> Result<Vec<Area>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_DELETED_AT} IS NULL
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
            "#
        );
        conn.prepare(&sql)?
            .query_map({}, mapper())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn select_updated_since(
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<Area>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_UPDATED_AT} > :updated_since
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
                LIMIT :limit
            "#
        );
        conn.prepare(&sql)?
            .query_map(
                named_params! {
                    ":updated_since": updated_since.format(&Rfc3339)?,
                    ":limit": limit.unwrap_or(i64::MAX),
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn select_by_search_query(search_query: &str, conn: &Connection) -> Result<Vec<Area>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE LOWER(json_extract({COL_TAGS}, '$.name')) LIKE '%' || UPPER(:query) || '%'
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
            "#
        );
        conn.prepare(&sql)?
            .query_map(named_params! { ":query": search_query }, mapper())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn select_by_id_or_alias(id_or_alias: &str, conn: &Connection) -> Result<Option<Area>> {
        match id_or_alias.parse::<i64>() {
            Ok(id) => Area::select_by_id(id, conn),
            Err(_) => Area::select_by_alias(id_or_alias, conn),
        }
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<Area>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_ID} = :{COL_ID}
            "#
        );
        conn.query_row(&sql, named_params! { ":id": id }, mapper())
            .optional()
            .map_err(Into::into)
    }

    pub fn select_by_alias(alias: &str, conn: &Connection) -> Result<Option<Area>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_ALIAS} = :alias
            "#
        );
        conn.query_row(&sql, named_params! { ":alias": alias }, mapper())
            .optional()
            .map_err(Into::into)
    }

    pub fn patch_tags(
        id: i64,
        tags: Map<String, Value>,
        conn: &Connection,
    ) -> Result<Option<Area>> {
        let sql = format!(
            r#"
                UPDATE {TABLE_NAME}
                SET {COL_TAGS} = json_patch({COL_TAGS}, json(:tags))
                WHERE {COL_ID} = :{COL_ID}
            "#
        );
        conn.execute(
            &sql,
            named_params! {
                ":id": id,
                ":tags": Value::from(tags),
            },
        )?;
        Area::select_by_id(id, conn)
    }

    pub fn remove_tag(id: i64, name: &str, conn: &Connection) -> Result<Option<Area>> {
        let sql = format!(
            r#"
                UPDATE {TABLE_NAME}
                SET {COL_TAGS} = json_remove({COL_TAGS}, :name)
                WHERE {COL_ID} = :{COL_ID}
            "#
        );
        conn.execute(
            &sql,
            named_params! {
                ":id": id,
                ":name": format!("$.{name}"),
            },
        )?;
        Area::select_by_id(id, conn)
    }

    pub fn set_updated_at(
        id: i64,
        updated_at: &OffsetDateTime,
        conn: &Connection,
    ) -> Result<Option<Area>> {
        let sql = format!(
            r#"
                UPDATE {TABLE_NAME}
                SET {COL_UPDATED_AT} = :{COL_UPDATED_AT}
                WHERE {COL_ID} = :{COL_ID}
            "#
        );
        conn.execute(
            &sql,
            named_params! {
                ":id": id,
                ":updated_at": updated_at.format(&Rfc3339)?,
            },
        )?;
        Area::select_by_id(id, conn)
    }

    pub fn set_deleted_at(
        id: i64,
        deleted_at: Option<OffsetDateTime>,
        conn: &Connection,
    ) -> Result<Option<Area>> {
        match deleted_at {
            Some(deleted_at) => {
                let sql = format!(
                    r#"
                        UPDATE {TABLE_NAME}
                        SET {COL_DELETED_AT} = :{COL_DELETED_AT}
                        WHERE {COL_ID} = :{COL_ID}
                    "#
                );
                conn.execute(
                    &sql,
                    named_params! {
                        ":id": id,
                        ":deleted_at": deleted_at.format(&Rfc3339)?,
                    },
                )?;
            }
            None => {
                let query = format!(
                    r#"
                        UPDATE {TABLE_NAME}
                        SET {COL_DELETED_AT} = NULL
                        WHERE {COL_ID} = :{COL_ID}
                    "#
                );
                conn.execute(&query, named_params! { ":id": id })?;
            }
        };
        Area::select_by_id(id, conn)
    }

    pub fn name(&self) -> String {
        self.tags
            .get("name")
            .map(|it| it.as_str().unwrap_or_default())
            .unwrap_or_default()
            .into()
    }

    pub fn alias(&self) -> String {
        self.tags
            .get("url_alias")
            .map(|it| it.as_str().unwrap_or_default())
            .unwrap_or_default()
            .into()
    }

    pub fn geo_json(&self) -> Result<GeoJson> {
        let geo_json = self.tags["geo_json"].clone();
        serde_json::to_string(&geo_json)?
            .parse()
            .map_err(Into::into)
    }

    pub fn geo_json_geometries(&self) -> Result<Vec<Geometry>> {
        let mut geometries: Vec<Geometry> = vec![];
        match self.geo_json()? {
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
        Ok(geometries)
    }

    #[cfg(test)]
    pub fn mock_tags() -> Map<String, Value> {
        let mut tags = Map::new();
        tags.insert(
            "geo_json".into(),
            GeoJson::Feature(geojson::Feature::default()).into(),
        );
        tags.insert("url_alias".into(), Value::String("alias".into()));
        tags
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
    use crate::{area::Area, test::mock_conn, Result};
    use geojson::{Feature, GeoJson};
    use serde_json::{json, Map};
    use time::ext::NumericalDuration;
    use time::{macros::datetime, OffsetDateTime};

    #[test]
    fn insert() -> Result<()> {
        let conn = mock_conn();
        let tags = Area::mock_tags();
        let res = Area::insert(tags.clone(), &conn)?.unwrap();
        assert_eq!(tags, res.tags);
        assert_eq!(res, Area::select_by_id(res.id, &conn)?.unwrap());
        Ok(())
    }

    #[test]
    fn insert_without_mandatory_tags() -> Result<()> {
        let conn = mock_conn();
        let res = Area::insert(Map::new(), &conn);
        assert!(res.is_err());
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = mock_conn();
        assert_eq!(
            vec![
                Area::insert(Area::mock_tags(), &conn)?.unwrap(),
                Area::insert(Area::mock_tags(), &conn)?.unwrap(),
                Area::insert(Area::mock_tags(), &conn)?.unwrap(),
            ],
            Area::select_all(&conn)?,
        );
        Ok(())
    }

    #[test]
    fn select_all_except_deleted() -> Result<()> {
        let conn = mock_conn();
        Area::insert(Area::mock_tags(), &conn)?.unwrap();
        Area::insert(Area::mock_tags(), &conn)?.unwrap();
        Area::insert(Area::mock_tags(), &conn)?.unwrap();
        Area::set_deleted_at(2, Some(OffsetDateTime::now_utc()), &conn)?;
        assert_eq!(2, Area::select_all_except_deleted(&conn)?.len(),);
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = mock_conn();
        let _area_1 = Area::insert(Area::mock_tags(), &conn)?.unwrap();
        let _area_1 = Area::set_updated_at(_area_1.id, &datetime!(2020-01-01 00:00 UTC), &conn)?;
        let area_2 = Area::insert(Area::mock_tags(), &conn)?.unwrap();
        let area_2 =
            Area::set_updated_at(area_2.id, &datetime!(2020-01-02 00:00 UTC), &conn)?.unwrap();
        let area_3 = Area::insert(Area::mock_tags(), &conn)?.unwrap();
        let area_3 =
            Area::set_updated_at(area_3.id, &datetime!(2020-01-03 00:00 UTC), &conn)?.unwrap();
        assert_eq!(
            vec![area_2, area_3],
            Area::select_updated_since(&datetime!(2020-01-01 00:00 UTC), None, &conn)?,
        );
        Ok(())
    }

    #[test]
    fn select_by_search_query() -> Result<()> {
        let conn = mock_conn();
        Area::insert(Area::mock_tags(), &conn)?.unwrap();
        Area::insert(Area::mock_tags(), &conn)?.unwrap();
        Area::insert(Area::mock_tags(), &conn)?.unwrap();
        Area::patch_tags(
            2,
            Map::from_iter([("name".into(), "sushi".into())].into_iter()),
            &conn,
        )?;
        assert_eq!(1, Area::select_by_search_query("sus", &conn)?.len());
        assert_eq!(1, Area::select_by_search_query("hi", &conn)?.len());
        assert_eq!(0, Area::select_by_search_query("sashimi", &conn)?.len());
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(Area::mock_tags(), &conn)?.unwrap();
        assert_eq!(area, Area::select_by_id(area.id, &conn)?.unwrap());
        Ok(())
    }

    #[test]
    fn select_by_alias() -> Result<()> {
        let conn = mock_conn();
        let url_alias = json!("url_alias_value");
        let mut tags = Map::new();
        tags.insert("url_alias".into(), url_alias.clone());
        tags.insert(
            "geo_json".into(),
            GeoJson::Feature(Feature::default()).into(),
        );
        Area::insert(tags, &conn)?;
        let area = Area::select_by_alias(url_alias.as_str().unwrap(), &conn)?;
        assert!(area.is_some());
        let area = area.unwrap();
        assert_eq!(url_alias, area.tags["url_alias"]);
        Ok(())
    }

    #[test]
    fn patch_tags() -> Result<()> {
        let conn = mock_conn();
        let tag_1_name = "tag_1_name";
        let tag_1_value = json!("tag_1_value");
        let tag_2_name = "tag_2_name";
        let tag_2_value = json!("tag_2_value");
        let mut tags = Area::mock_tags();
        tags.insert(tag_1_name.into(), tag_1_value.clone());
        let area = Area::insert(tags.clone(), &conn)?.unwrap();
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        tags.insert(tag_2_name.into(), tag_2_value.clone());
        let area = Area::patch_tags(area.id, tags, &conn)?.unwrap();
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        assert_eq!(tag_2_value, area.tags[tag_2_name]);
        Ok(())
    }

    #[test]
    fn set_updated_at() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(Area::mock_tags(), &conn)?.unwrap();
        let area = Area::set_updated_at(
            area.id,
            &OffsetDateTime::now_utc().checked_add(2.days()).unwrap(),
            &conn,
        )?
        .unwrap();
        assert!(area.updated_at > OffsetDateTime::now_utc().checked_add(1.days()).unwrap());
        Ok(())
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(Area::mock_tags(), &conn)?.unwrap();
        let area = Area::set_deleted_at(area.id, Some(OffsetDateTime::now_utc()), &conn)?.unwrap();
        assert!(area.deleted_at.is_some());
        let area = Area::set_deleted_at(area.id, None, &conn)?.unwrap();
        assert!(area.deleted_at.is_none());
        Ok(())
    }
}
