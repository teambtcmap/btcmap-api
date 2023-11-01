use std::collections::HashMap;

use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Row;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::Error;
use crate::Result;

#[derive(PartialEq, Debug)]
pub struct Area {
    pub id: i32,
    pub tags: HashMap<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Area {
    pub fn insert(tags: &HashMap<String, Value>, conn: &Connection) -> Result<Area> {
        conn.execute(
            "INSERT INTO area (tags) VALUES (json(:tags))",
            named_params! { ":tags": &serde_json::to_string(tags)? },
        )?;
        Ok(
            Area::select_by_id(conn.last_insert_rowid().try_into()?, &conn)?
                .ok_or(Error::DbTableRowNotFound)?,
        )
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
            ORDER BY updated_at, rowid
            LIMIT :limit
        "#;
        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! { ":limit": limit.unwrap_or(i32::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn select_updated_since(
        updated_since: &OffsetDateTime,
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
            ORDER BY updated_at, rowid
            LIMIT :limit
        "#;
        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! { ":updated_since": updated_since.format(&Rfc3339)?, ":limit": limit.unwrap_or(i32::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn select_by_id(id: i32, conn: &Connection) -> Result<Option<Area>> {
        let query = r#"
            SELECT
                rowid,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM area
            WHERE rowid = :id
        "#;
        Ok(conn
            .query_row(query, named_params! { ":id": id }, mapper())
            .optional()?)
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
        Ok(conn
            .query_row(query, named_params! { ":url_alias": url_alias }, mapper())
            .optional()?)
    }

    pub fn patch_tags(
        &self,
        tags: &HashMap<String, Value>,
        conn: &Connection,
    ) -> crate::Result<Area> {
        conn.execute(
            "UPDATE area SET tags = json_patch(tags, :tags) WHERE rowid = :id",
            named_params! { ":id": self.id, ":tags": &serde_json::to_string(tags)? },
        )?;
        Ok(Area::select_by_id(self.id, &conn)?.ok_or(Error::DbTableRowNotFound)?)
    }

    pub fn remove_tag(&self, tag: &str, conn: &Connection) -> Result<Area> {
        let query = r#"
            UPDATE area
            SET tags = json_remove(tags, :tag)
            WHERE rowid = :id
        "#;
        conn.execute(
            query,
            named_params! { ":id": self.id, ":tag": format!("$.{tag}") },
        )?;
        Ok(Area::select_by_id(self.id, &conn)?.ok_or(Error::DbTableRowNotFound)?)
    }

    #[cfg(test)]
    pub fn set_updated_at(&self, updated_at: &OffsetDateTime, conn: &Connection) -> Result<Area> {
        conn.execute(
            "UPDATE area SET updated_at = :updated_at WHERE rowid = :id",
            named_params! {
                ":id": self.id,
                ":updated_at": updated_at.format(&Rfc3339)?,
            },
        )?;
        Ok(Area::select_by_id(self.id, &conn)?.ok_or(Error::DbTableRowNotFound)?)
    }

    pub fn set_deleted_at(
        &self,
        deleted_at: Option<OffsetDateTime>,
        conn: &Connection,
    ) -> Result<Area> {
        match deleted_at {
            Some(deleted_at) => {
                conn.execute(
                    "UPDATE area SET deleted_at = :deleted_at WHERE rowid = :id",
                    named_params! { ":id": self.id, ":deleted_at": deleted_at.format(&Rfc3339)? },
                )?;
            }
            None => {
                conn.execute(
                    "UPDATE area SET deleted_at = NULL WHERE rowid = :id",
                    named_params! { ":id": self.id },
                )?;
            }
        };
        Ok(Area::select_by_id(self.id, &conn)?.ok_or(Error::DbTableRowNotFound)?)
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
    use time::{macros::datetime, OffsetDateTime};

    use crate::{
        test::{mock_conn, mock_tags},
        Result,
    };

    use super::Area;

    #[test]
    fn insert() -> Result<()> {
        let conn = mock_conn();
        let tags = mock_tags();
        let res = Area::insert(&tags, &conn)?;
        assert_eq!(tags, res.tags);
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = mock_conn();
        assert_eq!(
            vec![
                Area::insert(&HashMap::new(), &conn)?,
                Area::insert(&HashMap::new(), &conn)?,
                Area::insert(&HashMap::new(), &conn)?
            ],
            Area::select_all(None, &conn)?
        );
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = mock_conn();
        Area::insert(&mock_tags(), &conn)?
            .set_updated_at(&datetime!(2020-01-01 00:00 UTC), &conn)?;
        let expected_area_1 = Area::insert(&mock_tags(), &conn)?
            .set_updated_at(&datetime!(2020-01-02 00:00 UTC), &conn)?;
        let expected_area_2 = Area::insert(&mock_tags(), &conn)?
            .set_updated_at(&datetime!(2020-01-03 00:00 UTC), &conn)?;
        assert_eq!(
            vec![expected_area_1, expected_area_2],
            Area::select_updated_since(&datetime!(2020-01-01 00:00 UTC), None, &conn)?
        );
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(&HashMap::new(), &conn)?;
        let res = Area::select_by_id(area.id, &conn)?;
        assert!(res.is_some());
        assert_eq!(area, res.unwrap());
        Ok(())
    }

    #[test]
    fn select_by_url_alias() -> Result<()> {
        let conn = mock_conn();
        let url_alias = json!("url_alias_value");
        let mut tags = HashMap::new();
        tags.insert("url_alias".into(), url_alias.clone());
        Area::insert(&tags, &conn)?;
        let area = Area::select_by_url_alias(url_alias.as_str().unwrap(), &conn)?;
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
        let mut tags = HashMap::new();
        tags.insert(tag_1_name.into(), tag_1_value.clone());
        let area = Area::insert(&tags, &conn)?;
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        tags.insert(tag_2_name.into(), tag_2_value.clone());
        let area = area.patch_tags(&tags, &conn)?;
        assert_eq!(tag_1_value, area.tags[tag_1_name]);
        assert_eq!(tag_2_value, area.tags[tag_2_name]);
        Ok(())
    }

    #[test]
    fn remove_tag() -> Result<()> {
        let conn = mock_conn();
        let tag_name = "tag_name";
        let tag_value = json!("tag_value");
        let mut tags: HashMap<String, Value> = HashMap::new();
        tags.insert(tag_name.into(), tag_value.clone());
        let area = Area::insert(&tags, &conn)?;
        assert_eq!(tag_value, area.tags[tag_name].as_str().unwrap());
        let area = area.remove_tag(tag_name, &conn)?;
        assert!(area.tags.get(tag_name).is_none());
        Ok(())
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = mock_conn();
        let area = Area::insert(&HashMap::new(), &conn)?
            .set_deleted_at(Some(OffsetDateTime::now_utc()), &conn)?;
        assert!(area.deleted_at.is_some());
        let area = area.set_deleted_at(None, &conn)?;
        assert!(area.deleted_at.is_none());
        Ok(())
    }
}
