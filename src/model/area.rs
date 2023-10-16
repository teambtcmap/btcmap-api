use std::collections::HashMap;

use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Result;
use rusqlite::Row;
use serde_json::Value;
use time::OffsetDateTime;

pub struct Area {
    pub id: i32,
    pub tags: Option<HashMap<String, Value>>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Area {
    pub fn insert_or_replace(
        url_alias: &str,
        tags: Option<&HashMap<String, Value>>,
        conn: &Connection,
    ) -> crate::Result<()> {
        let area = Area::select_by_url_alias(url_alias, conn)?;

        match area {
            Some(area) => {
                conn.execute(
                    "UPDATE area SET tags = json_set(:tags, '$.url_alias', :url_alias) WHERE rowid = :id",
                    named_params! { ":id": area.id, ":url_alias": url_alias, ":tags": &serde_json::to_string(tags.unwrap_or(&HashMap::default()))? },
                )?;
            }
            None => {
                conn.execute(
                    "INSERT INTO area (tags) VALUES (json_set(:tags, '$.url_alias', :url_alias))",
                    named_params! { ":url_alias": url_alias, ":tags": &serde_json::to_string(tags.unwrap_or(&HashMap::default()))? },
                )?;
            }
        }

        Ok(())
    }

    pub fn select_all(conn: &Connection, limit: Option<i32>) -> Result<Vec<Area>> {
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
                full_mapper(),
            )?
            .collect::<Result<Vec<Area>, _>>()?)
    }

    pub fn select_updated_since(
        conn: &Connection,
        updated_since: &str,
        limit: Option<i32>,
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
                full_mapper(),
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

        let res = conn.query_row(
            query,
            named_params! { ":url_alias": url_alias },
            full_mapper(),
        );

        Ok(res.optional()?)
    }

    pub fn insert_tag(
        &self,
        tag_name: &str,
        tag_value: &str,
        conn: &Connection,
    ) -> crate::Result<()> {
        let tag_name = format!("$.{tag_name}");

        let query = r#"
            UPDATE area
            SET tags = json_set(tags, :tag_name, :tag_value)
            WHERE rowid = :id
        "#;

        conn.execute(
            query,
            named_params! { ":id": self.id, ":tag_name": tag_name, ":tag_value": tag_value },
        )?;

        Ok(())
    }

    pub fn insert_tag_json(
        &self,
        tag_name: &str,
        tag_value: &Value,
        conn: &Connection,
    ) -> crate::Result<()> {
        let tag_name = format!("$.{tag_name}");

        let query = r#"
            UPDATE area
            SET tags = json_set(tags, :tag_name, json(:tag_value))
            WHERE rowid = :id
        "#;

        conn.execute(
            query,
            named_params! { ":id": self.id, ":tag_name": tag_name, ":tag_value": serde_json::to_string(tag_value)? },
        )?;

        Ok(())
    }

    pub fn delete_tag(&self, tag: &str, conn: &Connection) -> crate::Result<()> {
        let tag = format!("$.{tag}");

        let query = r#"
            UPDATE area
            SET tags = json_remove(tags, :tag)
            WHERE rowid = :id
        "#;

        conn.execute(query, named_params! { ":id": self.id, ":tag": tag })?;

        Ok(())
    }

    pub fn delete(&self, conn: &Connection) -> crate::Result<()> {
        let query = r#"
            UPDATE area
            SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ')
            WHERE rowid = :id
        "#;

        conn.execute(query, named_params! { ":id": self.id })?;

        Ok(())
    }

    pub fn tag(&self, name: &str) -> &Value {
        match &self.tags {
            Some(tags) => tags.get(name).unwrap_or(&Value::Null),
            None => &Value::Null,
        }
    }
}

const fn full_mapper() -> fn(&Row) -> Result<Area> {
    |row: &Row| -> Result<Area> {
        let tags: Option<String> = row.get(1)?;
        let tags: Option<HashMap<String, Value>> =
            tags.map(|it| serde_json::from_str(&it).unwrap());

        Ok(Area {
            id: row.get(0)?,
            tags: tags,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
            deleted_at: row.get(4)?,
        })
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use serde_json::{Value, json};

    use crate::{command::db, Result};

    use super::Area;

    #[test]
    fn insert_or_replace() -> Result<()> {
        let conn = db::setup_connection()?;
        let url = "test";
        Area::insert_or_replace(url, None, &conn)?;
        let mut tags: HashMap<String, Value> = HashMap::new();
        tags.insert("foo".into(), Value::String("bar".into()));
        Area::insert_or_replace(url, Some(&tags), &conn)?;
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = db::setup_connection()?;
        Area::insert_or_replace("test_1", None, &conn)?;
        Area::insert_or_replace("test_2", None, &conn)?;
        Area::insert_or_replace("test_3", None, &conn)?;
        assert_eq!(3, Area::select_all(&conn, None)?.len());
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
            Area::select_updated_since(&conn, "2020-01-01T00:00:00Z", None)?.len()
        );
        Ok(())
    }

    #[test]
    fn select_by_url_alias() -> Result<()> {
        let conn = db::setup_connection()?;
        let alias = "test";
        Area::insert_or_replace(alias, None, &conn)?;
        let area = Area::select_by_url_alias(alias, &conn)?;
        assert!(area.is_some());
        assert_eq!(alias, area.unwrap().tag("url_alias").as_str().unwrap());
        Ok(())
    }

    #[test]
    fn insert_tag() -> Result<()> {
        let conn = db::setup_connection()?;
        let alias = "test";
        let tag_name = "foo";
        let tag_value = "bar";
        Area::insert_or_replace(alias, None, &conn)?;
        let area = Area::select_by_url_alias(alias, &conn)?.unwrap();
        area.insert_tag(tag_name, tag_value, &conn)?;
        let area = Area::select_by_url_alias(alias, &conn)?.unwrap();
        assert_eq!(tag_value, area.tag(tag_name).as_str().unwrap());
        Ok(())
    }

    #[test]
    fn insert_tag_json() -> Result<()> {
        let conn = db::setup_connection()?;
        let alias = "test";
        let tag_name = "foo";
        let tag_value = json!({"key": "value"});
        Area::insert_or_replace(alias, None, &conn)?;
        let area = Area::select_by_url_alias(alias, &conn)?.unwrap();
        area.insert_tag_json(tag_name, &tag_value, &conn)?;
        let area = Area::select_by_url_alias(alias, &conn)?.unwrap();
        assert_eq!(&tag_value, area.tag(tag_name));
        Ok(())
    }

    #[test]
    fn delete_tag() -> Result<()> {
        let conn = db::setup_connection()?;
        let alias = "test";
        let tag_name = "foo";
        let tag_value = "bar";
        let mut tags: HashMap<String, Value> = HashMap::new();
        tags.insert(tag_name.into(), tag_value.into());
        Area::insert_or_replace(alias, Some(&tags), &conn)?;
        let area = Area::select_by_url_alias(alias, &conn)?.unwrap();
        assert_eq!(tag_value, area.tag(tag_name).as_str().unwrap());
        area.delete_tag(tag_name, &conn)?;
        let area = Area::select_by_url_alias(alias, &conn)?.unwrap();
        assert!(area.tag(tag_name).is_null());
        Ok(())
    }

    #[test]
    fn delete() -> Result<()> {
        let conn = db::setup_connection()?;
        let alias = "test";
        Area::insert_or_replace(alias, None, &conn)?;
        let area = Area::select_by_url_alias(alias, &conn)?.unwrap();
        area.delete(&conn)?;
        let area = Area::select_by_url_alias(alias, &conn)?.unwrap();
        assert!(area.deleted_at.is_some());
        Ok(())
    }
}
