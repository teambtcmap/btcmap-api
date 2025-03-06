use crate::{error::Error, Result};
use deadpool_sqlite::Pool;
use geojson::{GeoJson, Geometry};
use rusqlite::{named_params, Connection, Row};
use serde_json::{Map, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const TABLE_NAME: &str = "area";

#[derive(Debug, PartialEq, Eq)]
pub struct Area {
    pub id: i64,
    pub tags: Map<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

const COL_ID: &str = "id";
const COL_TAGS: &str = "tags";
const COL_ALIAS: &str = "alias";
const COL_UPDATED_AT: &str = "updated_at";
const COL_DELETED_AT: &str = "deleted_at";
const MAPPER_PROJECTION: &str = "id, tags, created_at, updated_at, deleted_at";

impl Area {
    pub fn insert(tags: Map<String, Value>, conn: &Connection) -> Result<Area> {
        let alias = tags
            .get("url_alias")
            .cloned()
            .ok_or(Error::invalid_input("url_alias is missing"))?;
        let alias = alias
            .as_str()
            .ok_or(Error::invalid_input("url_alias is not a string"))?;
        let _ = tags
            .get("geo_json")
            .ok_or(Error::invalid_input("geo_json is missing"))?;
        let geo_json = tags["geo_json"].clone();
        serde_json::to_string(&geo_json)?
            .parse::<GeoJson>()
            .map_err(|_| Error::invalid_input("invalid geo_json"))?;
        let sql = format!(
            r#"
                INSERT INTO {TABLE_NAME} ({COL_TAGS}, {COL_ALIAS})
                VALUES (json(:{COL_TAGS}), :{COL_ALIAS});
            "#
        );
        conn.execute(
            &sql,
            named_params! { ":tags": Value::from(tags), ":alias": alias },
        )?;
        Area::select_by_id(conn.last_insert_rowid(), conn)
    }

    pub async fn select_all_async(pool: &Pool) -> Result<Vec<Area>> {
        pool.get()
            .await?
            .interact(|conn| Area::select_all(conn))
            .await?
    }

    pub fn select_all(conn: &Connection) -> Result<Vec<Area>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                ORDER BY {COL_UPDATED_AT}, {COL_ID};
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
                ORDER BY {COL_UPDATED_AT}, {COL_ID};
            "#
        );
        conn.prepare(&sql)?
            .query_map({}, mapper())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub async fn select_updated_since_async(
        updated_since: OffsetDateTime,
        limit: Option<i64>,
        pool: &Pool,
    ) -> Result<Vec<Area>> {
        pool.get()
            .await?
            .interact(move |conn| Area::select_updated_since(updated_since, limit, conn))
            .await?
    }

    pub fn select_updated_since(
        updated_since: OffsetDateTime,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<Area>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_UPDATED_AT} > :updated_since
                ORDER BY {COL_UPDATED_AT}, {COL_ID}
                LIMIT :limit;
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

    pub async fn select_by_search_query_async(
        search_query: impl Into<String>,
        pool: &Pool,
    ) -> Result<Vec<Self>> {
        let search_query = search_query.into();
        pool.get()
            .await?
            .interact(move |conn| Self::select_by_search_query(&search_query, conn))
            .await?
    }

    pub fn select_by_search_query(
        search_query: impl Into<String>,
        conn: &Connection,
    ) -> Result<Vec<Self>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE LOWER(json_extract({COL_TAGS}, '$.name')) LIKE '%' || UPPER(:query) || '%'
                ORDER BY {COL_UPDATED_AT}, {COL_ID};
            "#
        );
        conn.prepare(&sql)?
            .query_map(named_params! { ":query": search_query.into() }, mapper())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub async fn select_by_id_or_alias_async(
        id_or_alias: impl Into<String>,
        pool: &Pool,
    ) -> Result<Area> {
        let id_or_alias = id_or_alias.into();
        pool.get()
            .await?
            .interact(|conn| Area::select_by_id_or_alias(id_or_alias, conn))
            .await?
    }

    pub fn select_by_id_or_alias(
        id_or_alias: impl Into<String>,
        conn: &Connection,
    ) -> Result<Area> {
        let id_or_alias = id_or_alias.into();
        match id_or_alias.parse::<i64>() {
            Ok(id) => Area::select_by_id(id, conn),
            Err(_) => Area::select_by_alias(&id_or_alias, conn),
        }
    }

    pub async fn select_by_id_async(id: i64, pool: &Pool) -> Result<Area> {
        pool.get()
            .await?
            .interact(move |conn| Area::select_by_id(id, conn))
            .await?
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Area> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_ID} = :{COL_ID};
            "#
        );
        conn.query_row(&sql, named_params! { ":id": id }, mapper())
            .map_err(Into::into)
    }

    pub async fn select_by_alias_async(alias: impl Into<String>, pool: &Pool) -> Result<Area> {
        let alias = alias.into();
        pool.get()
            .await?
            .interact(|conn| Area::select_by_alias(alias, conn))
            .await?
    }

    pub fn select_by_alias(alias: impl Into<String>, conn: &Connection) -> Result<Area> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_ALIAS} = :{COL_ALIAS};
            "#
        );
        conn.query_row(&sql, named_params! { ":alias": alias.into() }, mapper())
            .map_err(Into::into)
    }

    pub async fn patch_tags_async(
        area_id: i64,
        tags: Map<String, Value>,
        pool: &Pool,
    ) -> Result<Area> {
        pool.get()
            .await?
            .interact(move |conn| Area::patch_tags(area_id, tags, conn))
            .await?
    }

    pub fn patch_tags(area_id: i64, tags: Map<String, Value>, conn: &Connection) -> Result<Area> {
        let sql = format!(
            r#"
                UPDATE {TABLE_NAME}
                SET {COL_TAGS} = json_patch({COL_TAGS}, json(:tags))
                WHERE {COL_ID} = :{COL_ID};
            "#
        );
        conn.execute(
            &sql,
            named_params! {
                ":id": area_id,
                ":tags": Value::from(tags),
            },
        )?;
        Area::select_by_id(area_id, conn)
    }

    pub fn remove_tag(
        area_id: i64,
        tag_name: impl Into<String>,
        conn: &Connection,
    ) -> Result<Area> {
        let tag_name = tag_name.into();
        let sql = format!(
            r#"
                UPDATE {TABLE_NAME}
                SET {COL_TAGS} = json_remove({COL_TAGS}, :name)
                WHERE {COL_ID} = :{COL_ID};
            "#
        );
        conn.execute(
            &sql,
            named_params! {
                ":id": area_id,
                ":name": format!("$.{tag_name}"),
            },
        )?;
        Area::select_by_id(area_id, conn)
    }

    pub fn set_updated_at(
        area_id: i64,
        updated_at: &OffsetDateTime,
        conn: &Connection,
    ) -> Result<Area> {
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
                ":id": area_id,
                ":updated_at": updated_at.format(&Rfc3339)?,
            },
        )?;
        Area::select_by_id(area_id, conn)
    }

    pub fn set_deleted_at(
        area_id: i64,
        deleted_at: Option<OffsetDateTime>,
        conn: &Connection,
    ) -> Result<Area> {
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
                        ":id": area_id,
                        ":deleted_at": deleted_at.format(&Rfc3339)?,
                    },
                )?;
            }
            None => {
                let sql = format!(
                    r#"
                        UPDATE {TABLE_NAME}
                        SET {COL_DELETED_AT} = NULL
                        WHERE {COL_ID} = :{COL_ID}
                    "#
                );
                conn.execute(&sql, named_params! { ":id": area_id })?;
            }
        };
        Area::select_by_id(area_id, conn)
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

    pub fn geo_json_geometries(&self) -> Result<Vec<Geometry>> {
        let mut geometries: Vec<Geometry> = vec![];
        let geo_json = self.tags["geo_json"].clone();
        let geo_json: GeoJson = serde_json::to_string(&geo_json)?.parse()?;
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
    use serde_json::{json, Map};
    use time::ext::NumericalDuration;
    use time::{macros::datetime, OffsetDateTime};

    #[test]
    fn insert() -> Result<()> {
        let conn = mock_conn();
        let tags = Area::mock_tags();
        let area = Area::insert(tags, &conn)?;
        assert_eq!(area, Area::select_by_id(area.id, &conn)?);
        Ok(())
    }

    #[test]
    fn insert_without_alias() -> Result<()> {
        let conn = mock_conn();
        let mut tags = Area::mock_tags();
        tags.remove("url_alias");
        assert!(Area::insert(tags, &conn).is_err());
        Ok(())
    }

    #[test]
    fn insert_without_geo_json() -> Result<()> {
        let conn = mock_conn();
        let mut tags = Area::mock_tags();
        tags.remove("geo_json");
        assert!(Area::insert(tags, &conn).is_err());
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = mock_conn();
        assert_eq!(
            vec![
                Area::insert(Area::mock_tags(), &conn)?,
                Area::insert(Area::mock_tags(), &conn)?,
                Area::insert(Area::mock_tags(), &conn)?,
            ],
            Area::select_all(&conn)?,
        );
        Ok(())
    }

    #[test]
    fn select_all_except_deleted() -> Result<()> {
        let conn = mock_conn();
        let mut areas = vec![
            Area::insert(Area::mock_tags(), &conn)?,
            Area::insert(Area::mock_tags(), &conn)?,
            Area::insert(Area::mock_tags(), &conn)?,
        ];
        Area::set_deleted_at(areas.remove(1).id, Some(OffsetDateTime::now_utc()), &conn)?;
        assert_eq!(areas, Area::select_all_except_deleted(&conn)?);
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = mock_conn();
        let _area_1 = Area::insert(Area::mock_tags(), &conn)?;
        let _area_1 = Area::set_updated_at(_area_1.id, &datetime!(2020-01-01 00:00 UTC), &conn)?;
        let area_2 = Area::insert(Area::mock_tags(), &conn)?;
        let area_2 = Area::set_updated_at(area_2.id, &datetime!(2020-01-02 00:00 UTC), &conn)?;
        let area_3 = Area::insert(Area::mock_tags(), &conn)?;
        let area_3 = Area::set_updated_at(area_3.id, &datetime!(2020-01-03 00:00 UTC), &conn)?;
        assert_eq!(
            vec![area_2, area_3],
            Area::select_updated_since(datetime!(2020-01-01 00:00 UTC), None, &conn)?,
        );
        Ok(())
    }

    #[test]
    fn select_by_search_query() -> Result<()> {
        let conn = mock_conn();
        let areas = vec![
            Area::insert(Area::mock_tags(), &conn)?,
            Area::insert(Area::mock_tags(), &conn)?,
            Area::insert(Area::mock_tags(), &conn)?,
        ];
        Area::patch_tags(
            areas[1].id,
            Map::from_iter([("name".into(), "sushi".into())].into_iter()),
            &conn,
        )?;
        assert_eq!(1, Area::select_by_search_query("sus", &conn)?.len());
        assert_eq!(1, Area::select_by_search_query("hi", &conn)?.len());
        assert_eq!(0, Area::select_by_search_query("sashimi", &conn)?.len());
        Ok(())
    }

    #[test]
    fn select_by_id_or_alias() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(Area::mock_tags(), &conn)?;
        assert_eq!(
            area,
            Area::select_by_id_or_alias(area.id.to_string(), &conn)?
        );
        assert_eq!(area, Area::select_by_id_or_alias(area.alias(), &conn)?);
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(Area::mock_tags(), &conn)?;
        assert_eq!(area, Area::select_by_id(area.id, &conn)?);
        Ok(())
    }

    #[test]
    fn select_by_alias() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(Area::mock_tags(), &conn)?;
        assert_eq!(area, Area::select_by_id_or_alias(area.alias(), &conn)?);
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
        let area = Area::insert(tags.clone(), &conn)?;
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        tags.insert(tag_2_name.into(), tag_2_value.clone());
        let area = Area::patch_tags(area.id, tags, &conn)?;
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        assert_eq!(tag_2_value, area.tags[tag_2_name]);
        Ok(())
    }

    #[test]
    fn set_updated_at() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(Area::mock_tags(), &conn)?;
        let area = Area::set_updated_at(
            area.id,
            &OffsetDateTime::now_utc().checked_add(2.days()).unwrap(),
            &conn,
        )?;
        assert!(area.updated_at > OffsetDateTime::now_utc().checked_add(1.days()).unwrap());
        Ok(())
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(Area::mock_tags(), &conn)?;
        let area = Area::set_deleted_at(area.id, Some(OffsetDateTime::now_utc()), &conn)?;
        assert!(area.deleted_at.is_some());
        let area = Area::set_deleted_at(area.id, None, &conn)?;
        assert!(area.deleted_at.is_none());
        Ok(())
    }

    #[test]
    fn name() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(Area::mock_tags(), &conn)?;
        assert_eq!(String::default(), area.name());
        let name = "foo";
        let area = Area::patch_tags(
            area.id,
            Map::from_iter([("name".into(), name.into())].into_iter()),
            &conn,
        )?;
        assert_eq!(name, area.name());
        Ok(())
    }

    #[test]
    fn alias() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(Area::mock_tags(), &conn)?;
        assert_eq!(area.tags["url_alias"], area.alias());
        Ok(())
    }
}
