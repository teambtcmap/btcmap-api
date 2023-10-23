use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;

use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Row;
use serde_json::Value;
use time::OffsetDateTime;

use crate::Result;

pub struct Event {
    pub id: i32,
    pub user_id: i32,
    pub element_id: String,
    pub r#type: String,
    pub tags: HashMap<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

impl Event {
    pub fn insert(
        user_id: i32,
        element_id: &str,
        r#type: &str,
        conn: &Connection,
    ) -> crate::Result<()> {
        let query = r#"
            INSERT INTO event (
                user_id,
                element_id, 
                type
            ) VALUES (
                :user_id,
                :element_id,
                :type
            )
        "#;

        conn.execute(
            query,
            named_params! {
                ":user_id": user_id,
                ":element_id": element_id,
                ":type": r#type,
            },
        )?;

        sleep(Duration::from_millis(10));

        Ok(())
    }

    pub fn select_all(limit: Option<i32>, conn: &Connection) -> Result<Vec<Event>> {
        let query = r#"
            SELECT
                rowid,
                user_id,
                element_id,
                type,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM event
            ORDER BY updated_at
            LIMIT :limit
        "#;

        Ok(conn
            .prepare(query)?
            .query_map(
                named_params! { ":limit": limit.unwrap_or(std::i32::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<Event>, _>>()?)
    }

    pub fn select_updated_since(
        updated_since: &str,
        limit: Option<i32>,
        conn: &Connection,
    ) -> Result<Vec<Event>> {
        let query = r#"
            SELECT
                rowid,
                user_id,
                element_id,
                type,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM event
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
            .collect::<Result<Vec<Event>, _>>()?)
    }

    pub fn select_by_id(id: i32, conn: &Connection) -> Result<Option<Event>> {
        let query = r#"
            SELECT
                rowid,
                user_id,
                element_id,
                type,
                tags,
                created_at,
                updated_at,
                deleted_at
            FROM event 
            WHERE rowid = :id
        "#;

        Ok(conn
            .query_row(query, named_params! { ":id": id }, mapper())
            .optional()?)
    }

    pub fn merge_tags(id: i32, tags: &HashMap<String, Value>, conn: &Connection) -> Result<()> {
        let query = r#"
            UPDATE event
            SET tags = json_patch(tags, :tags)
            WHERE rowid = :id
        "#;

        conn.execute(
            query,
            named_params! { ":id": id, ":tags": &serde_json::to_string(tags)? },
        )?;

        Ok(())
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<Event> {
    |row: &Row| -> rusqlite::Result<Event> {
        let tags: String = row.get(4)?;

        Ok(Event {
            id: row.get(0)?,
            user_id: row.get(1)?,
            element_id: row.get(2)?,
            r#type: row.get(3)?,
            tags: serde_json::from_str(&tags).unwrap(),
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            deleted_at: row.get(7)?,
        })
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{command::db, Result};

    use super::Event;

    #[test]
    fn insert() -> Result<()> {
        let conn = db::setup_connection()?;
        Event::insert(1, "node:1", "create", &conn)?;
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = db::setup_connection()?;
        Event::insert(1, "node:1", "type1", &conn)?;
        Event::insert(2, "node:2", "type2", &conn)?;
        Event::insert(3, "node:3", "type3", &conn)?;
        assert_eq!(3, Event::select_all(None, &conn)?.len());
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = db::setup_connection()?;
        conn.execute(
            "INSERT INTO event (user_id, element_id, type, updated_at) VALUES (1, 'node:1', 'test', '2020-01-01T00:00:00Z')",
            [],
        )?;
        conn.execute(
            "INSERT INTO event (user_id, element_id, type, updated_at) VALUES (1, 'node:1', 'test', '2020-01-02T00:00:00Z')",
            [],
        )?;
        conn.execute(
            "INSERT INTO event (user_id, element_id, type, updated_at) VALUES (1, 'node:1', 'test', '2020-01-03T00:00:00Z')",
            [],
        )?;
        assert_eq!(
            2,
            Event::select_updated_since("2020-01-01T00:00:00Z", None, &conn,)?.len()
        );
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = db::setup_connection()?;
        Event::insert(1, "node:1", "test", &conn)?;
        assert!(Event::select_by_id(1, &conn)?.is_some());
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
        Event::insert(1, "node:1", "test", &conn)?;
        let event = Event::select_by_id(1, &conn)?.unwrap();
        assert!(event.tags.is_empty());
        Event::merge_tags(1, &tags, &conn)?;
        let event = Event::select_by_id(1, &conn)?.unwrap();
        assert_eq!(1, event.tags.len());
        tags.insert(tag_2_name.into(), tag_2_value.into());
        Event::merge_tags(1, &tags, &conn)?;
        let event = Event::select_by_id(1, &conn)?.unwrap();
        assert_eq!(2, event.tags.len());
        Ok(())
    }
}
