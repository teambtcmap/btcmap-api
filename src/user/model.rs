use crate::{osm::osm::OsmUser, Error, Result};
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use serde_json::{Map, Value};
use std::collections::HashMap;
#[cfg(not(test))]
use std::thread::sleep;
#[cfg(not(test))]
use std::time::Duration;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
#[cfg(test)]
use tracing::debug;

pub struct User {
    pub id: i64,
    pub osm_data: OsmUser,
    pub tags: Map<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl User {
    pub fn insert(id: i64, osm_data: &OsmUser, conn: &Connection) -> Result<User> {
        let query = r#"
            INSERT INTO user (
                rowid,
                osm_data
            ) VALUES (
                :id,
                :osm_data
            )
        "#;
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            query,
            named_params! {
                ":id": id,
                ":osm_data": serde_json::to_string(osm_data)?,
            },
        )?;

        Ok(User::select_by_id(conn.last_insert_rowid(), &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn select_all(limit: Option<i64>, conn: &Connection) -> Result<Vec<User>> {
        let query = r#"
            SELECT
                id,
                osm_data,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM user
            ORDER BY updated_at, id
            LIMIT :limit
        "#;

        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! { ":limit": limit.unwrap_or(i64::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn select_updated_since(
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<User>> {
        let query = r#"
            SELECT
                rowid,
                osm_data,
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
                named_params! {
                    ":updated_since": updated_since.format(&Rfc3339)?,
                    ":limit": limit.unwrap_or(i64::MAX)
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<User>> {
        let query = r#"
            SELECT
                rowid,
                osm_data,
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

    pub fn set_tag(&self, name: &str, value: &Value, conn: &Connection) -> Result<User> {
        User::_set_tag(self.id, name, value, conn)
    }

    pub fn _set_tag(id: i64, name: &str, value: &Value, conn: &Connection) -> Result<User> {
        let mut patch_set = HashMap::new();
        patch_set.insert(name.into(), value.clone());
        User::patch_tags(id, &patch_set, conn)
    }

    pub fn patch_tags(
        id: i64,
        tags: &HashMap<String, Value>,
        conn: &Connection,
    ) -> crate::Result<User> {
        let query = r#"
            UPDATE user
            SET tags = json_patch(tags, :tags)
            WHERE rowid = :id
        "#;
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            query,
            named_params! { ":id": id, ":tags": &serde_json::to_string(tags)? },
        )?;
        Ok(User::select_by_id(id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    pub fn set_osm_data(id: i64, osm_data: &OsmUser, conn: &Connection) -> Result<()> {
        let query = r#"
            UPDATE user
            SET osm_data = json(:osm_data)
            WHERE rowid = :id
        "#;
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            query,
            named_params! {
                ":id": id,
                ":osm_data": serde_json::to_string(osm_data)?,
            },
        )?;

        Ok(())
    }

    #[cfg(test)]
    pub fn _set_updated_at(
        id: i64,
        updated_at: &OffsetDateTime,
        conn: &Connection,
    ) -> Result<User> {
        let query = format!(
            r#"
                UPDATE user
                SET updated_at = :updated_at
                WHERE rowid = :id
            "#
        );
        debug!(query);
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            &query,
            named_params! {
                ":id": id,
                ":updated_at": updated_at.format(&Rfc3339)?,
            },
        )?;
        Ok(User::select_by_id(id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<User> {
    |row: &Row| -> rusqlite::Result<User> {
        let osm_data: String = row.get(1)?;
        let tags: String = row.get(2)?;

        Ok(User {
            id: row.get(0)?,
            osm_data: serde_json::from_str(&osm_data).unwrap(),
            tags: serde_json::from_str(&tags).unwrap(),
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{osm::osm::OsmUser, test::mock_conn, user::User, Result};
    use std::collections::HashMap;
    use time::macros::datetime;

    #[test]
    fn insert() -> Result<()> {
        let conn = mock_conn();
        User::insert(1, &OsmUser::mock(), &conn)?;
        let users = User::select_all(None, &conn)?;
        assert_eq!(1, users.len());
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = mock_conn();
        User::insert(1, &OsmUser::mock(), &conn)?;
        User::insert(2, &OsmUser::mock(), &conn)?;
        User::insert(3, &OsmUser::mock(), &conn)?;
        let reports = User::select_all(None, &conn)?;
        assert_eq!(3, reports.len());
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = mock_conn();
        conn.execute(
            "INSERT INTO user (rowid, osm_data, updated_at) VALUES (1, json(?), '2020-01-01T00:00:00Z')",
            [serde_json::to_string(&OsmUser::mock())?],
        )?;
        conn.execute(
            "INSERT INTO user (rowid, osm_data, updated_at) VALUES (2, json(?), '2020-01-02T00:00:00Z')",
            [serde_json::to_string(&OsmUser::mock())?],
        )?;
        conn.execute(
            "INSERT INTO user (rowid, osm_data, updated_at) VALUES (3, json(?), '2020-01-03T00:00:00Z')",
            [serde_json::to_string(&OsmUser::mock())?],
        )?;
        assert_eq!(
            2,
            User::select_updated_since(&datetime!(2020-01-01 00:00:00 UTC), None, &conn)?.len()
        );
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        User::insert(1, &OsmUser::mock(), &conn)?;
        assert!(User::select_by_id(1, &conn)?.is_some());
        Ok(())
    }

    #[test]
    fn merge_tags() -> Result<()> {
        let conn = mock_conn();
        let tag_1_name = "foo";
        let tag_1_value = "bar";
        let tag_2_name = "qwerty";
        let tag_2_value = "test";
        let mut tags = HashMap::new();
        tags.insert(tag_1_name.into(), tag_1_value.into());
        User::insert(1, &OsmUser::mock(), &conn)?;
        let user = User::select_by_id(1, &conn)?.unwrap();
        assert!(user.tags.is_empty());
        User::patch_tags(1, &tags, &conn)?;
        let user = User::select_by_id(1, &conn)?.unwrap();
        assert_eq!(1, user.tags.len());
        tags.insert(tag_2_name.into(), tag_2_value.into());
        User::patch_tags(1, &tags, &conn)?;
        let user = User::select_by_id(1, &conn)?.unwrap();
        assert_eq!(2, user.tags.len());
        Ok(())
    }

    #[test]
    fn set_osm_data() -> Result<()> {
        let conn = mock_conn();
        let user = OsmUser {
            id: 1,
            ..OsmUser::mock()
        };
        User::insert(user.id, &user, &conn)?;
        let user = OsmUser {
            id: 2,
            ..OsmUser::mock()
        };
        User::set_osm_data(1, &user, &conn)?;
        let user = User::select_by_id(1, &conn)?.unwrap();
        assert_eq!(2, user.osm_data.id);
        Ok(())
    }
}
