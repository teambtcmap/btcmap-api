use crate::{error::Error, Result};
use deadpool_sqlite::Pool;
use geojson::{GeoJson, Geometry};
use rusqlite::{named_params, Connection, Row};
use serde_json::{Map, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const TABLE_NAME: &str = "area";

enum Columns {
    Id,
    Alias,
    Tags,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::Alias => "alias",
            Columns::Tags => "tags",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }

    fn projection_full() -> String {
        vec![
            Self::Id,
            Self::Alias,
            Self::Tags,
            Self::CreatedAt,
            Self::UpdatedAt,
            Self::DeletedAt,
        ]
        .iter()
        .map(Self::as_str)
        .collect::<Vec<_>>()
        .join(", ")
    }

    fn mapper_full() -> fn(&Row) -> rusqlite::Result<Area> {
        |row: &Row| -> rusqlite::Result<Area> {
            let tags: String = row.get(2)?;
            Ok(Area {
                id: row.get(0)?,
                alias: row.get(1)?,
                tags: serde_json::from_str(&tags).unwrap(),
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                deleted_at: row.get(5)?,
            })
        }
    }
}

pub struct Area {
    pub id: i64,
    pub alias: String,
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
const MAPPER_PROJECTION: &str = "id, alias, tags, created_at, updated_at, deleted_at";

impl Area {
    pub async fn insert(tags: Map<String, Value>, pool: &Pool) -> Result<Self> {
        pool.get()
            .await?
            .interact(|conn| Self::_insert(tags, conn))
            .await?
    }

    fn _insert(tags: Map<String, Value>, conn: &Connection) -> Result<Self> {
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
            .query_map({}, Columns::mapper_full())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub async fn select_all_except_deleted_async(pool: &Pool) -> Result<Vec<Self>> {
        pool.get()
            .await?
            .interact(|conn| Self::select_all_except_deleted(conn))
            .await?
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
            .query_map({}, Columns::mapper_full())?
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
                Columns::mapper_full(),
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
            .query_map(
                named_params! { ":query": search_query.into() },
                Columns::mapper_full(),
            )?
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
        conn.query_row(&sql, named_params! { ":id": id }, Columns::mapper_full())
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
        conn.query_row(
            &sql,
            named_params! { ":alias": alias.into() },
            Columns::mapper_full(),
        )
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

    pub async fn set_updated_at_async(
        area_id: i64,
        updated_at: OffsetDateTime,
        pool: &Pool,
    ) -> Result<Area> {
        pool.get()
            .await?
            .interact(move |conn| Self::set_updated_at(area_id, &updated_at, conn))
            .await?
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

    pub async fn set_deleted_at_async(
        area_id: i64,
        deleted_at: Option<OffsetDateTime>,
        pool: &Pool,
    ) -> Result<Area> {
        pool.get()
            .await?
            .interact(move |conn| Self::set_deleted_at(area_id, deleted_at, conn))
            .await?
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

#[cfg(test)]
mod test {
    use crate::test::mock_pool;
    use crate::{area::Area, Result};
    use actix_web::test;
    use serde_json::{json, Map};
    use time::ext::NumericalDuration;
    use time::{macros::datetime, OffsetDateTime};

    #[test]
    async fn insert() -> Result<()> {
        let pool = mock_pool().await;
        let tags = Area::mock_tags();
        let area = Area::insert(tags, &pool).await?;
        assert_eq!(area.id, Area::select_by_id_async(area.id, &pool).await?.id);
        Ok(())
    }

    #[test]
    async fn insert_without_alias() -> Result<()> {
        let pool = mock_pool().await;
        let mut tags = Area::mock_tags();
        tags.remove("url_alias");
        assert!(Area::insert(tags, &pool).await.is_err());
        Ok(())
    }

    #[test]
    async fn insert_without_geo_json() -> Result<()> {
        let pool = mock_pool().await;
        let mut tags = Area::mock_tags();
        tags.remove("geo_json");
        assert!(Area::insert(tags, &pool).await.is_err());
        Ok(())
    }

    #[test]
    async fn select_all() -> Result<()> {
        let pool = mock_pool().await;
        Area::insert(Area::mock_tags(), &pool).await?;
        Area::insert(Area::mock_tags(), &pool).await?;
        Area::insert(Area::mock_tags(), &pool).await?;
        assert_eq!(3, Area::select_all_async(&pool).await?.len());
        Ok(())
    }

    #[test]
    async fn select_all_except_deleted() -> Result<()> {
        let pool = mock_pool().await;
        let mut areas = vec![
            Area::insert(Area::mock_tags(), &pool).await?,
            Area::insert(Area::mock_tags(), &pool).await?,
            Area::insert(Area::mock_tags(), &pool).await?,
        ];
        Area::set_deleted_at_async(areas.remove(1).id, Some(OffsetDateTime::now_utc()), &pool)
            .await?;
        assert_eq!(2, Area::select_all_except_deleted_async(&pool).await?.len());
        Ok(())
    }

    #[test]
    async fn select_updated_since() -> Result<()> {
        let pool = mock_pool().await;
        let _area_1 = Area::insert(Area::mock_tags(), &pool).await?;
        let _area_1 =
            Area::set_updated_at_async(_area_1.id, datetime!(2020-01-01 00:00 UTC), &pool).await?;
        let area_2 = Area::insert(Area::mock_tags(), &pool).await?;
        let _area_2 =
            Area::set_updated_at_async(area_2.id, datetime!(2020-01-02 00:00 UTC), &pool).await?;
        let area_3 = Area::insert(Area::mock_tags(), &pool).await?;
        let _area_3 =
            Area::set_updated_at_async(area_3.id, datetime!(2020-01-03 00:00 UTC), &pool).await?;
        assert_eq!(
            2,
            Area::select_updated_since_async(datetime!(2020-01-01 00:00 UTC), None, &pool)
                .await?
                .len(),
        );
        Ok(())
    }

    #[test]
    async fn select_by_search_query() -> Result<()> {
        let pool = mock_pool().await;
        let areas = vec![
            Area::insert(Area::mock_tags(), &pool).await?,
            Area::insert(Area::mock_tags(), &pool).await?,
            Area::insert(Area::mock_tags(), &pool).await?,
        ];
        Area::patch_tags_async(
            areas[1].id,
            Map::from_iter([("name".into(), "sushi".into())].into_iter()),
            &pool,
        )
        .await?;
        assert_eq!(
            1,
            Area::select_by_search_query_async("sus", &pool)
                .await?
                .len()
        );
        assert_eq!(
            1,
            Area::select_by_search_query_async("hi", &pool).await?.len()
        );
        assert_eq!(
            0,
            Area::select_by_search_query_async("sashimi", &pool)
                .await?
                .len()
        );
        Ok(())
    }

    #[test]
    async fn select_by_id_or_alias() -> Result<()> {
        let pool = mock_pool().await;
        let area = Area::insert(Area::mock_tags(), &pool).await?;
        assert_eq!(
            area.id,
            Area::select_by_id_or_alias_async(area.id.to_string(), &pool)
                .await?
                .id
        );
        assert_eq!(
            area.id,
            Area::select_by_id_or_alias_async(area.alias(), &pool)
                .await?
                .id,
        );
        Ok(())
    }

    #[test]
    async fn select_by_id() -> Result<()> {
        let pool = mock_pool().await;
        let area = Area::insert(Area::mock_tags(), &pool).await?;
        assert_eq!(area.id, Area::select_by_id_async(area.id, &pool).await?.id);
        Ok(())
    }

    #[test]
    async fn select_by_alias() -> Result<()> {
        let pool = mock_pool().await;
        let area = Area::insert(Area::mock_tags(), &pool).await?;
        assert_eq!(
            area.id,
            Area::select_by_id_or_alias_async(area.alias(), &pool)
                .await?
                .id,
        );
        Ok(())
    }

    #[test]
    async fn patch_tags() -> Result<()> {
        let pool = mock_pool().await;
        let tag_1_name = "tag_1_name";
        let tag_1_value = json!("tag_1_value");
        let tag_2_name = "tag_2_name";
        let tag_2_value = json!("tag_2_value");
        let mut tags = Area::mock_tags();
        tags.insert(tag_1_name.into(), tag_1_value.clone());
        let area = Area::insert(tags.clone(), &pool).await?;
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        tags.insert(tag_2_name.into(), tag_2_value.clone());
        let area = Area::patch_tags_async(area.id, tags, &pool).await?;
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        assert_eq!(tag_2_value, area.tags[tag_2_name]);
        Ok(())
    }

    #[test]
    async fn set_updated_at() -> Result<()> {
        let pool = mock_pool().await;
        let area = Area::insert(Area::mock_tags(), &pool).await?;
        let area = Area::set_updated_at_async(
            area.id,
            OffsetDateTime::now_utc().checked_add(2.days()).unwrap(),
            &pool,
        )
        .await?;
        assert!(area.updated_at > OffsetDateTime::now_utc().checked_add(1.days()).unwrap());
        Ok(())
    }

    #[test]
    async fn set_deleted_at() -> Result<()> {
        let pool = mock_pool().await;
        let area = Area::insert(Area::mock_tags(), &pool).await?;
        let area =
            Area::set_deleted_at_async(area.id, Some(OffsetDateTime::now_utc()), &pool).await?;
        assert!(area.deleted_at.is_some());
        let area = Area::set_deleted_at_async(area.id, None, &pool).await?;
        assert!(area.deleted_at.is_none());
        Ok(())
    }

    #[test]
    async fn name() -> Result<()> {
        let pool = mock_pool().await;
        let area = Area::insert(Area::mock_tags(), &pool).await?;
        assert_eq!(String::default(), area.name());
        let name = "foo";
        let area = Area::patch_tags_async(
            area.id,
            Map::from_iter([("name".into(), name.into())].into_iter()),
            &pool,
        )
        .await?;
        assert_eq!(name, area.name());
        Ok(())
    }

    #[test]
    async fn alias() -> Result<()> {
        let pool = mock_pool().await;
        let area = Area::insert(Area::mock_tags(), &pool).await?;
        assert_eq!(area.tags["url_alias"], area.alias());
        Ok(())
    }
}
