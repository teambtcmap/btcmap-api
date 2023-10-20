use std::collections::HashMap;

use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Row;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::Result;

pub struct Area {
    pub id: i32,
    pub tags: HashMap<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Area {
    pub fn insert(tags: &HashMap<String, Value>, conn: &Connection) -> crate::Result<()> {
        conn.execute(
            "INSERT INTO area (tags) VALUES (json(:tags))",
            named_params! { ":tags": &serde_json::to_string(tags)? },
        )?;

        Ok(())
    }

    pub fn select_all(limit: Option<i32>, conn: &Connection) -> Result<Vec<Area>> {
        let query = r#"
            SELECT
                rowid,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM area
            ORDER BY updated_at
            LIMIT :limit
        "#;

        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! { ":limit": limit.unwrap_or(std::i32::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<Area>, _>>()?)
    }

    pub fn select_updated_since(
        updated_since: &str,
        limit: Option<i32>,
        conn: &Connection,
    ) -> Result<Vec<Area>> {
        let query = r#"
            SELECT
                rowid,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM area
            WHERE updated_at > :updated_since
            ORDER BY updated_at
            LIMIT :limit
        "#;

        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! { ":updated_since": updated_since, ":limit": limit.unwrap_or(std::i32::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<Area>, _>>()?)
    }

    pub fn select_by_url_alias(url_alias: &str, conn: &Connection) -> Result<Option<Area>> {
        let query = r#"
            SELECT
                rowid,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM area
            WHERE json_extract(tags, '$.url_alias') = :url_alias
        "#;

        let res = conn.query_row(query, named_params! { ":url_alias": url_alias }, mapper());

        Ok(res.optional()?)
    }

    pub fn merge_tags(
        id: i32,
        tags: &HashMap<String, Value>,
        conn: &Connection,
    ) -> crate::Result<()> {
        let query = r#"
            UPDATE area
            SET tags = json_patch(tags, :tags)
            WHERE rowid = :id
        "#;

        conn.execute(
            query,
            named_params! { ":id": id, ":tags": &serde_json::to_string(tags)? },
        )?;

        Ok(())
    }

    pub fn insert_tag_as_str(
        id: i32,
        name: &str,
        value: &str,
        conn: &Connection,
    ) -> crate::Result<()> {
        let name = format!("$.{name}");

        let query = r#"
            UPDATE area
            SET tags = json_set(tags, :name, :value)
            WHERE rowid = :id
        "#;

        conn.execute(
            query,
            named_params! { ":id": id, ":name": name, ":value": value },
        )?;

        Ok(())
    }

    pub fn insert_tag_as_json_obj(
        id: i32,
        name: &str,
        value: &HashMap<String, Value>,
        conn: &Connection,
    ) -> crate::Result<()> {
        let name = format!("$.{name}");

        let query = r#"
            UPDATE area
            SET tags = json_set(tags, :name, json(:value))
            WHERE rowid = :id
        "#;

        conn.execute(
            query,
            named_params! { ":id": id, ":name": name, ":value": serde_json::to_string(value)? },
        )?;

        Ok(())
    }

    pub fn delete_tag(id: i32, tag: &str, conn: &Connection) -> crate::Result<()> {
        let tag = format!("$.{tag}");

        let query = r#"
            UPDATE area
            SET tags = json_remove(tags, :tag)
            WHERE rowid = :id
        "#;

        conn.execute(query, named_params! { ":id": id, ":tag": tag })?;

        Ok(())
    }

    pub fn set_deleted_at(
        id: i32,
        deleted_at: Option<OffsetDateTime>,
        conn: &Connection,
    ) -> Result<()> {
        let deleted_at = deleted_at.map(|it| it.format(&Rfc3339).unwrap());

        match deleted_at {
            Some(deleted_at) => {
                let query = r#"
                    UPDATE area
                    SET deleted_at = :deleted_at
                    WHERE rowid = :id
                "#;

                conn.execute(
                    query,
                    named_params! { ":id": id, ":deleted_at": deleted_at },
                )?;
            }
            None => {
                let query = r#"
                    UPDATE area
                    SET deleted_at = NULL
                    WHERE rowid = :id
                "#;

                conn.execute(query, named_params! { ":id": id })?;
            }
        };

        Ok(())
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
    use std::collections::HashMap;

    use serde_json::{json, Value};
    use time::OffsetDateTime;

    use crate::{command::db, Result};

    use super::Area;

    #[test]
    fn insert() -> Result<()> {
        let conn = db::setup_connection()?;
        let url_alias = "test";
        let mut tags: HashMap<String, Value> = HashMap::new();
        tags.insert("foo".into(), Value::String("bar".into()));
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        Area::insert(&tags, &conn)?;
        let area = Area::select_by_url_alias(url_alias, &conn)?.unwrap();
        assert_eq!(2, area.tags.len());
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = db::setup_connection()?;
        Area::insert(&HashMap::new(), &conn)?;
        Area::insert(&HashMap::new(), &conn)?;
        Area::insert(&HashMap::new(), &conn)?;
        assert_eq!(3, Area::select_all(None, &conn)?.len());
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = db::setup_connection()?;
        conn.execute(
            "INSERT INTO area (rowid, updated_at) VALUES (1, '2020-01-01T00:00:00Z')",
            [],
        )?;
        conn.execute(
            "INSERT INTO area (rowid, updated_at) VALUES (2, '2020-01-02T00:00:00Z')",
            [],
        )?;
        conn.execute(
            "INSERT INTO area (rowid, updated_at) VALUES (3, '2020-01-03T00:00:00Z')",
            [],
        )?;
        assert_eq!(
            2,
            Area::select_updated_since("2020-01-01T00:00:00Z", None, &conn,)?.len()
        );
        Ok(())
    }

    #[test]
    fn select_by_url_alias() -> Result<()> {
        let conn = db::setup_connection()?;
        let url_alias = "test";
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        Area::insert(&tags, &conn)?;
        let area = Area::select_by_url_alias(url_alias, &conn)?;
        assert_eq!(url_alias, area.unwrap().tags["url_alias"].as_str().unwrap());
        Ok(())
    }

    #[test]
    fn merge_tags() -> Result<()> {
        let conn = db::setup_connection()?;
        let url_alias = "test";
        let tag_1_name = "foo";
        let tag_1_value = "bar";
        let tag_2_name = "qwerty";
        let tag_2_value = "test";
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), url_alias.into());
        tags.insert(tag_1_name.into(), tag_1_value.into());
        Area::insert(&tags, &conn)?;
        let area = Area::select_by_url_alias(url_alias, &conn)?.unwrap();
        assert_eq!(tag_1_value, area.tags[tag_1_name].as_str().unwrap());
        tags.insert(tag_2_name.into(), tag_2_value.into());
        Area::merge_tags(area.id, &tags, &conn)?;
        let area = Area::select_by_url_alias(url_alias, &conn)?.unwrap();
        assert_eq!(tag_1_value, area.tags[tag_1_name].as_str().unwrap());
        assert_eq!(tag_2_value, area.tags[tag_2_name].as_str().unwrap());
        Ok(())
    }

    #[test]
    fn insert_tag_as_str() -> Result<()> {
        let conn = db::setup_connection()?;
        let url_alias = "test";
        let tag_name = "foo";
        let tag_value = "bar";
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        Area::insert(&tags, &conn)?;
        let area = Area::select_by_url_alias(url_alias, &conn)?.unwrap();
        Area::insert_tag_as_str(area.id, tag_name, tag_value, &conn)?;
        let area = Area::select_by_url_alias(url_alias, &conn)?.unwrap();
        assert_eq!(tag_value, area.tags[tag_name].as_str().unwrap());
        Ok(())
    }

    #[test]
    fn insert_tag_as_json_obj() -> Result<()> {
        let conn = db::setup_connection()?;
        let url_alias = "test";
        let tag_name = "foo";
        let tag_value: HashMap<String, Value> = serde_json::from_value(json!({"key": "value"}))?;
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), Value::String(url_alias.into()));
        Area::insert(&tags, &conn)?;
        let area = Area::select_by_url_alias(url_alias, &conn)?.unwrap();
        Area::insert_tag_as_json_obj(area.id, tag_name, &tag_value, &conn)?;
        let area = Area::select_by_url_alias(url_alias, &conn)?.unwrap();
        assert!(area.tags[tag_name].is_object());
        Ok(())
    }

    #[test]
    fn delete_tag() -> Result<()> {
        let conn = db::setup_connection()?;
        let url_alias = "test";
        let tag_name = "foo";
        let tag_value = "bar";
        let mut tags: HashMap<String, Value> = HashMap::new();
        tags.insert("url_alias".into(), url_alias.into());
        tags.insert(tag_name.into(), tag_value.into());
        Area::insert(&tags, &conn)?;
        let area = Area::select_by_url_alias(url_alias, &conn)?.unwrap();
        assert_eq!(tag_value, area.tags[tag_name].as_str().unwrap());
        Area::delete_tag(area.id, tag_name, &conn)?;
        let area = Area::select_by_url_alias(url_alias, &conn)?.unwrap();
        assert!(area.tags.get(tag_name).is_none());
        Ok(())
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = db::setup_connection()?;
        Area::insert(&HashMap::new(), &conn)?;
        let area = &Area::select_all(None, &conn)?[0];
        Area::set_deleted_at(area.id, Some(OffsetDateTime::now_utc()), &conn)?;
        let area = &Area::select_all(None, &conn)?[0];
        assert!(area.deleted_at.is_some());
        Area::set_deleted_at(area.id, None, &conn)?;
        let area = &Area::select_all(None, &conn)?[0];
        assert!(area.deleted_at.is_none());
        Ok(())
    }
}
