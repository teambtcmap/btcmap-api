use super::schema;
use super::schema::Area;
use super::schema::Columns;
use crate::Result;
use geojson::GeoJson;
use rusqlite::params;
use rusqlite::Connection;
use serde_json::{Map, Value};
use std::i64;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub fn insert(tags: Map<String, Value>, conn: &Connection) -> Result<Area> {
    let alias = tags
        .get("url_alias")
        .cloned()
        .ok_or("url_alias is missing")?;
    let alias = alias.as_str().ok_or("url_alias is not a string")?;
    let _ = tags.get("geo_json").ok_or("geo_json is missing")?;
    let geo_json = tags["geo_json"].clone();
    serde_json::to_string(&geo_json)?
        .parse::<GeoJson>()
        .map_err(|_| "invalid geo_json")?;
    let sql = format!(
        r#"
                INSERT INTO {table} ({alias}, {tags})
                VALUES (?1, json(?2))
                RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        tags = Columns::Tags.as_str(),
        alias = Columns::Alias.as_str(),
        projection = Area::projection(),
    );
    conn.query_row(&sql, params![alias, Value::from(tags)], Area::mapper())
        .map_err(Into::into)
}

pub fn select(
    updated_since: Option<OffsetDateTime>,
    include_deleted: bool,
    limit: Option<i64>,
    conn: &Connection,
) -> Result<Vec<Area>> {
    let updated_since_sql = match updated_since {
        Some(updated_since) => format!(
            "AND {updated_at} > '{updated_since}'",
            updated_at = Columns::UpdatedAt.as_str(),
            updated_since = updated_since.format(&Rfc3339)?
        ),
        None => String::new(),
    };
    let include_deleted_sql = if include_deleted {
        "".into()
    } else {
        format!(
            "AND {deleted_at} IS NULL",
            deleted_at = Columns::DeletedAt.as_str()
        )
    };
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE 1
            {updated_since_sql}
            {include_deleted_sql}
            ORDER BY {updated_at}, {id}
            LIMIT {limit}
        "#,
        projection = Area::projection(),
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
        limit = limit.unwrap_or(i64::MAX)
    );
    conn.prepare(&sql)?
        .query_map({}, Area::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_search_query(
    search_query: impl Into<String>,
    conn: &Connection,
) -> Result<Vec<Area>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE LOWER(json_extract({tags}, '$.name')) LIKE '%' || UPPER(?1) || '%'
            ORDER BY {updated_at}, {id}
        "#,
        projection = Area::projection(),
        table = schema::TABLE_NAME,
        tags = Columns::Tags.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(params![search_query.into()], Area::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id_or_alias(id_or_alias: impl Into<String>, conn: &Connection) -> Result<Area> {
    let id_or_alias = id_or_alias.into();
    match id_or_alias.parse::<i64>() {
        Ok(id) => select_by_id(id, conn),
        Err(_) => select_by_alias(&id_or_alias, conn),
    }
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<Area> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = Area::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![id], Area::mapper())
        .map_err(Into::into)
}

pub fn select_by_alias(alias: impl Into<String>, conn: &Connection) -> Result<Area> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {alias} = ?1
        "#,
        projection = Area::projection(),
        table = schema::TABLE_NAME,
        alias = Columns::Alias.as_str(),
    );
    conn.query_row(&sql, params![alias.into()], Area::mapper())
        .map_err(Into::into)
}

pub fn patch_tags(area_id: i64, tags: Map<String, Value>, conn: &Connection) -> Result<Area> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {tags} = json_patch({tags}, json(?2))
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        tags = Columns::Tags.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(
        &sql,
        params! {
            area_id,
            Value::from(tags),
        },
    )?;
    select_by_id(area_id, conn)
}

pub fn remove_tag(area_id: i64, tag_name: impl Into<String>, conn: &Connection) -> Result<Area> {
    let tag_name = tag_name.into();
    let sql = format!(
        r#"
            UPDATE {table}
            SET {tags} = json_remove({tags}, ?2)
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        tags = Columns::Tags.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![area_id, format!("$.{tag_name}")])?;
    select_by_id(area_id, conn)
}

#[cfg(test)]
pub fn set_updated_at(id: i64, updated_at: &OffsetDateTime, conn: &Connection) -> Result<Area> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {updated_at} = ?2
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![id, updated_at.format(&Rfc3339)?,])?;
    select_by_id(id, conn)
}

pub fn set_bbox(
    id: i64,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    conn: &Connection,
) -> Result<Area> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {bbox_west} = ?2, {bbox_south} = ?3, {bbox_east} = ?4, {bbox_north} = ?5
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        bbox_west = Columns::BboxWest.as_str(),
        bbox_south = Columns::BboxSouth.as_str(),
        bbox_east = Columns::BboxEast.as_str(),
        bbox_north = Columns::BboxNorth.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![id, west, south, east, north])?;
    select_by_id(id, conn)
}

pub fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    conn: &Connection,
) -> Result<Area> {
    match deleted_at {
        Some(deleted_at) => {
            let sql = format!(
                r#"
                    UPDATE {table}
                    SET {deleted_at} = ?2
                    WHERE {id} = ?1
                "#,
                table = schema::TABLE_NAME,
                deleted_at = Columns::DeletedAt.as_str(),
                id = Columns::Id.as_str(),
            );
            conn.execute(&sql, params![id, deleted_at.format(&Rfc3339)?,])?;
        }
        None => {
            let sql = format!(
                r#"
                        UPDATE {table}
                        SET {deleted_at} = NULL
                        WHERE {id} = ?1
                "#,
                table = schema::TABLE_NAME,
                deleted_at = Columns::DeletedAt.as_str(),
                id = Columns::Id.as_str(),
            );
            conn.execute(&sql, params![id])?;
        }
    };
    select_by_id(id, conn)
}

#[cfg(test)]
mod test {
    use crate::db::area::schema::Area;
    use crate::db::test::conn;
    use crate::Result;
    use serde_json::{json, Map};
    use time::ext::NumericalDuration;
    use time::macros::datetime;
    use time::{Duration, OffsetDateTime};

    #[test]
    fn insert() -> Result<()> {
        let conn = conn();
        let tags = Area::mock_tags();
        let area = super::insert(tags, &conn)?;
        assert_eq!(area.id, super::select_by_id(area.id, &conn)?.id);
        Ok(())
    }

    #[test]
    fn insert_without_alias() -> Result<()> {
        let conn = conn();
        let mut tags = Area::mock_tags();
        tags.remove("url_alias");
        assert!(super::insert(tags, &conn).is_err());
        Ok(())
    }

    #[test]
    fn insert_without_geo_json() -> Result<()> {
        let conn = conn();
        let mut tags = Area::mock_tags();
        tags.remove("geo_json");
        assert!(super::insert(tags, &conn).is_err());
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = conn();
        super::insert(Area::mock_tags(), &conn)?;
        super::insert(Area::mock_tags(), &conn)?;
        super::insert(Area::mock_tags(), &conn)?;
        assert_eq!(3, super::select(None, true, None, &conn)?.len());
        Ok(())
    }

    #[test]
    fn select_all_should_sort_by_updated_at_asc() -> Result<()> {
        let conn = conn();
        let area_1 = super::insert(Area::mock_tags(), &conn)?;
        let area_1 = super::set_updated_at(
            area_1.id,
            &(OffsetDateTime::now_utc() - Duration::hours(3)),
            &conn,
        )?;
        let area_2 = super::insert(Area::mock_tags(), &conn)?;
        let area_2 = super::set_updated_at(
            area_2.id,
            &(OffsetDateTime::now_utc() - Duration::hours(1)),
            &conn,
        )?;
        let area_3 = super::insert(Area::mock_tags(), &conn)?;
        assert_eq!(3, super::select(None, true, None, &conn)?.len());
        let area_3 = super::set_updated_at(
            area_3.id,
            &(OffsetDateTime::now_utc() - Duration::hours(2)),
            &conn,
        )?;
        let areas = super::select(None, false, None, &conn)?;
        assert_eq!(area_1.id, areas[0].id);
        assert_eq!(area_3.id, areas[1].id);
        assert_eq!(area_2.id, areas[2].id);
        Ok(())
    }

    #[test]
    fn select_all_except_deleted() -> Result<()> {
        let conn = conn();
        let mut areas = vec![
            super::insert(Area::mock_tags(), &conn)?,
            super::insert(Area::mock_tags(), &conn)?,
            super::insert(Area::mock_tags(), &conn)?,
        ];
        super::set_deleted_at(areas.remove(1).id, Some(OffsetDateTime::now_utc()), &conn)?;
        assert_eq!(2, super::select(None, false, None, &conn)?.len());
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = conn();
        let _area_1 = super::insert(Area::mock_tags(), &conn)?;
        let _area_1 = super::set_updated_at(_area_1.id, &datetime!(2020-01-01 00:00 UTC), &conn)?;
        let area_2 = super::insert(Area::mock_tags(), &conn)?;
        let _area_2 = super::set_updated_at(area_2.id, &datetime!(2020-01-02 00:00 UTC), &conn)?;
        let area_3 = super::insert(Area::mock_tags(), &conn)?;
        let _area_3 = super::set_updated_at(area_3.id, &datetime!(2020-01-03 00:00 UTC), &conn)?;
        assert_eq!(
            2,
            super::select(Some(datetime!(2020-01-01 00:00 UTC)), false, None, &conn)?.len(),
        );
        Ok(())
    }

    #[test]
    fn select_by_search_query() -> Result<()> {
        let conn = conn();
        let areas = vec![
            super::insert(Area::mock_tags(), &conn)?,
            super::insert(Area::mock_tags(), &conn)?,
            super::insert(Area::mock_tags(), &conn)?,
        ];
        super::patch_tags(
            areas[1].id,
            Map::from_iter([("name".into(), "sushi".into())].into_iter()),
            &conn,
        )?;
        assert_eq!(1, super::select_by_search_query("sus", &conn)?.len());
        assert_eq!(1, super::select_by_search_query("hi", &conn)?.len());
        assert_eq!(0, super::select_by_search_query("sashimi", &conn)?.len());
        Ok(())
    }

    #[test]
    fn select_by_id_or_alias() -> Result<()> {
        let conn = conn();
        let area = super::insert(Area::mock_tags(), &conn)?;
        assert_eq!(
            area.id,
            super::select_by_id_or_alias(area.id.to_string(), &conn)?.id
        );
        assert_eq!(
            area.id,
            super::select_by_id_or_alias(area.alias(), &conn)?.id,
        );
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = conn();
        let area = super::insert(Area::mock_tags(), &conn)?;
        assert_eq!(area.id, super::select_by_id(area.id, &conn)?.id);
        Ok(())
    }

    #[test]
    fn select_by_alias() -> Result<()> {
        let conn = conn();
        let area = super::insert(Area::mock_tags(), &conn)?;
        assert_eq!(
            area.id,
            super::select_by_id_or_alias(area.alias(), &conn)?.id,
        );
        Ok(())
    }

    #[test]
    fn patch_tags() -> Result<()> {
        let conn = conn();
        let tag_1_name = "tag_1_name";
        let tag_1_value = json!("tag_1_value");
        let tag_2_name = "tag_2_name";
        let tag_2_value = json!("tag_2_value");
        let mut tags = Area::mock_tags();
        tags.insert(tag_1_name.into(), tag_1_value.clone());
        let area = super::insert(tags.clone(), &conn)?;
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        tags.insert(tag_2_name.into(), tag_2_value.clone());
        let area = super::patch_tags(area.id, tags, &conn)?;
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        assert_eq!(tag_2_value, area.tags[tag_2_name]);
        Ok(())
    }

    #[test]
    fn set_updated_at() -> Result<()> {
        let conn = conn();
        let area = super::insert(Area::mock_tags(), &conn)?;
        let area = super::set_updated_at(
            area.id,
            &OffsetDateTime::now_utc().saturating_add(2.days()),
            &conn,
        )?;
        assert!(area.updated_at > OffsetDateTime::now_utc().saturating_add(1.days()));
        Ok(())
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = conn();
        let area = super::insert(Area::mock_tags(), &conn)?;
        let area = super::set_deleted_at(area.id, Some(OffsetDateTime::now_utc()), &conn)?;
        assert!(area.deleted_at.is_some());
        let area = super::set_deleted_at(area.id, None, &conn)?;
        assert!(area.deleted_at.is_none());
        Ok(())
    }

    #[test]
    fn name() -> Result<()> {
        let conn = conn();
        let area = super::insert(Area::mock_tags(), &conn)?;
        assert_eq!(String::default(), area.name());
        let name = "foo";
        let area = super::patch_tags(
            area.id,
            Map::from_iter([("name".into(), name.into())].into_iter()),
            &conn,
        )?;
        assert_eq!(name, area.name());
        Ok(())
    }

    #[test]
    fn alias() -> Result<()> {
        let conn = conn();
        let area = super::insert(Area::mock_tags(), &conn)?;
        assert_eq!(area.tags["url_alias"], area.alias());
        Ok(())
    }
}
