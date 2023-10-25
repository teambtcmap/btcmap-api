use std::collections::HashMap;

use rusqlite::{named_params, Connection, OptionalExtension, Row};
use serde_json::Value;
use time::OffsetDateTime;

use crate::{service::osm::OsmUser, Result};

pub struct User {
    pub id: i32,
    pub osm_json: OsmUser,
    pub tags: HashMap<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl User {
    pub fn insert(id: i32, osm_json: &OsmUser, conn: &Connection) -> Result<()> {
        let query = r#"
            INSERT INTO user (
                rowid,
                osm_json
            ) VALUES (
                :id,
                :osm_json
            )
        "#;

        conn.execute(
            query,
            named_params! {
                ":id": id,
                ":osm_json": serde_json::to_string(osm_json)?,
            },
        )?;

        Ok(())
    }

    pub fn select_all(limit: Option<i32>, conn: &Connection) -> Result<Vec<User>> {
        let query = r#"
            SELECT
                rowid,
                osm_json,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM user
            ORDER BY updated_at, rowid
            LIMIT :limit
        "#;

        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! { ":limit": limit.unwrap_or(std::i32::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn select_updated_since(
        updated_since: &str,
        limit: Option<i32>,
        conn: &Connection,
    ) -> Result<Vec<User>> {
        let query = r#"
            SELECT
                rowid,
                osm_json,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM user
            WHERE updated_at > :updated_since
            ORDER BY updated_at, rowid
            LIMIT :limit
        "#;

        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! { ":updated_since": updated_since, ":limit": limit.unwrap_or(std::i32::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn select_by_id(id: i32, conn: &Connection) -> Result<Option<User>> {
        let query = r#"
            SELECT
                rowid,
                osm_json,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM user
            WHERE rowid = :id
        "#;

        Ok(conn
            .query_row(query, named_params! { ":id": id }, mapper())
            .optional()?)
    }

    pub fn merge_tags(
        id: i32,
        tags: &HashMap<String, Value>,
        conn: &Connection,
    ) -> crate::Result<()> {
        let query = r#"
            UPDATE user
            SET tags = json_patch(tags, :tags)
            WHERE rowid = :id
        "#;

        conn.execute(
            query,
            named_params! { ":id": id, ":tags": &serde_json::to_string(tags)? },
        )?;

        Ok(())
    }

    pub fn set_osm_json(id: i32, osm_json: &OsmUser, conn: &Connection) -> Result<()> {
        let query = r#"
            UPDATE user
            SET osm_json = json(:osm_json)
            WHERE rowid = :id
        "#;

        conn.execute(
            query,
            named_params! {
                ":id": id,
                ":osm_json": serde_json::to_string(osm_json)?,
            },
        )?;

        Ok(())
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<User> {
    |row: &Row| -> rusqlite::Result<User> {
        let osm_json: String = row.get(1)?;
        let osm_json: OsmUser = serde_json::from_str(&osm_json).unwrap();

        let tags: String = row.get(2)?;
        let tags: HashMap<String, Value> = serde_json::from_str(&tags).unwrap_or_default();

        Ok(User {
            id: row.get(0)?,
            osm_json,
            tags,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
        })
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{command::db, model::User, service::osm::OsmUser, Result};

    #[test]
    fn insert() -> Result<()> {
        let conn = db::setup_connection()?;
        User::insert(1, &OsmUser::mock(), &conn)?;
        let users = User::select_all(None, &conn)?;
        assert_eq!(1, users.len());
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = db::setup_connection()?;
        User::insert(1, &OsmUser::mock(), &conn)?;
        User::insert(2, &OsmUser::mock(), &conn)?;
        User::insert(3, &OsmUser::mock(), &conn)?;
        let reports = User::select_all(None, &conn)?;
        assert_eq!(3, reports.len());
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = db::setup_connection()?;
        conn.execute(
            "INSERT INTO user (rowid, osm_json, updated_at) VALUES (1, json(?), '2020-01-01T00:00:00Z')",
            [serde_json::to_string(&OsmUser::mock())?],
        )?;
        conn.execute(
            "INSERT INTO user (rowid, osm_json, updated_at) VALUES (2, json(?), '2020-01-02T00:00:00Z')",
            [serde_json::to_string(&OsmUser::mock())?],
        )?;
        conn.execute(
            "INSERT INTO user (rowid, osm_json, updated_at) VALUES (3, json(?), '2020-01-03T00:00:00Z')",
            [serde_json::to_string(&OsmUser::mock())?],
        )?;
        assert_eq!(
            2,
            User::select_updated_since("2020-01-01T00:00:00Z", None, &conn)?.len()
        );
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = db::setup_connection()?;
        User::insert(1, &OsmUser::mock(), &conn)?;
        assert!(User::select_by_id(1, &conn)?.is_some());
        Ok(())
    }

    #[test]
    fn merge_tags() -> Result<()> {
        let conn = db::setup_connection()?;
        let tag_1_name = "foo";
        let tag_1_value = "bar";
        let tag_2_name = "qwerty";
        let tag_2_value = "test";
        let mut tags = HashMap::new();
        tags.insert(tag_1_name.into(), tag_1_value.into());
        User::insert(1, &OsmUser::mock(), &conn)?;
        let user = User::select_by_id(1, &conn)?.unwrap();
        assert!(user.tags.is_empty());
        User::merge_tags(1, &tags, &conn)?;
        let user = User::select_by_id(1, &conn)?.unwrap();
        assert_eq!(1, user.tags.len());
        tags.insert(tag_2_name.into(), tag_2_value.into());
        User::merge_tags(1, &tags, &conn)?;
        let user = User::select_by_id(1, &conn)?.unwrap();
        assert_eq!(2, user.tags.len());
        Ok(())
    }

    #[test]
    fn set_osm_json() -> Result<()> {
        let conn = db::setup_connection()?;
        let user = OsmUser {
            id: 1,
            ..OsmUser::mock()
        };
        User::insert(user.id, &user, &conn)?;
        let user = OsmUser {
            id: 2,
            ..OsmUser::mock()
        };
        User::set_osm_json(1, &user, &conn)?;
        let user = User::select_by_id(1, &conn)?.unwrap();
        assert_eq!(2, user.osm_json.id);
        Ok(())
    }
}
