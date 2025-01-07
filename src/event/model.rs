use crate::Error;
use crate::Result;
use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Row;
use serde_json::Value;
use std::collections::HashMap;
#[cfg(not(test))]
use std::thread::sleep;
#[cfg(not(test))]
use std::time::Duration;
use std::time::Instant;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::debug;
use tracing::info;

#[derive(Debug, Eq, PartialEq)]
pub struct Event {
    pub id: i64,
    pub user_id: i64,
    pub element_id: i64,
    pub element_osm_type: String,
    pub element_osm_id: i64,
    pub r#type: String,
    pub tags: HashMap<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

const TABLE: &str = "event";
const _ALL_COLUMNS: &str =
    "rowid, user_id, element_id, type, tags, created_at, updated_at, deleted_at";
const COL_ROWID: &str = "rowid";
const COL_USER_ID: &str = "user_id";
const COL_ELEMENT_ID: &str = "element_id";
const COL_TYPE: &str = "type";
const COL_TAGS: &str = "tags";
const COL_CREATED_AT: &str = "created_at";
const COL_UPDATED_AT: &str = "updated_at";
const COL_DELETED_AT: &str = "deleted_at";

impl Event {
    pub fn insert(user_id: i64, element_id: i64, r#type: &str, conn: &Connection) -> Result<Event> {
        let query = format!(
            r#"
                INSERT INTO {TABLE} (
                    {COL_USER_ID},
                    {COL_ELEMENT_ID},
                    {COL_TYPE}
                ) VALUES (
                    :user_id,
                    :element_id,
                    :type
                )
            "#
        );
        debug!(query);
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            &query,
            named_params! {
                ":user_id": user_id,
                ":element_id": element_id,
                ":type": r#type,
            },
        )?;
        Event::select_by_id(conn.last_insert_rowid(), conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    pub fn select_all(
        sort_order: Option<String>,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<Event>> {
        let sort_order = sort_order.unwrap_or("ASC".into());
        let query = format!(
            r#"
                SELECT
                    ev.{COL_ROWID},
                    ev.{COL_USER_ID},
                    ev.{COL_ELEMENT_ID},
                    json_extract(el.overpass_data, '$.type'),
                    json_extract(el.overpass_data, '$.id'),
                    ev.{COL_TYPE},
                    ev.{COL_TAGS},
                    ev.{COL_CREATED_AT},
                    ev.{COL_UPDATED_AT},
                    ev.{COL_DELETED_AT}
                FROM {TABLE} ev
                LEFT JOIN element el on el.rowid = ev.{COL_ELEMENT_ID}
                ORDER BY ev.{COL_UPDATED_AT} {sort_order}, ev.{COL_ROWID} {sort_order}
                LIMIT :limit
            "#
        );
        debug!(query);
        Ok(conn
            .prepare(&query)?
            .query_map(
                named_params! { ":limit": limit.unwrap_or(i64::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn select_by_type(
        r#type: &str,
        sort_order: Option<String>,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<Event>> {
        let sort_order = sort_order.unwrap_or("ASC".into());
        let query = format!(
            r#"
                SELECT
                    ev.{COL_ROWID},
                    ev.{COL_USER_ID},
                    ev.{COL_ELEMENT_ID},
                    json_extract(el.overpass_data, '$.type'),
                    json_extract(el.overpass_data, '$.id'),
                    ev.{COL_TYPE},
                    ev.{COL_TAGS},
                    ev.{COL_CREATED_AT},
                    ev.{COL_UPDATED_AT},
                    ev.{COL_DELETED_AT}
                FROM {TABLE} ev
                LEFT JOIN element el on el.rowid = ev.{COL_ELEMENT_ID}
                WHERE ev.{COL_TYPE} = :type
                ORDER BY ev.{COL_UPDATED_AT} {sort_order}, ev.{COL_ROWID} {sort_order}
                LIMIT :limit
            "#
        );
        debug!(query);
        Ok(conn
            .prepare(&query)?
            .query_map(
                named_params! { ":type": r#type, ":limit": limit.unwrap_or(i64::MAX) },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn select_updated_since(
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
        conn: &Connection,
    ) -> Result<Vec<Event>> {
        let query = format!(
            r#"
                SELECT
                    ev.{COL_ROWID},
                    ev.{COL_USER_ID},
                    ev.{COL_ELEMENT_ID},
                    json_extract(el.overpass_data, '$.type'),
                    json_extract(el.overpass_data, '$.id'),
                    ev.{COL_TYPE},
                    ev.{COL_TAGS},
                    ev.{COL_CREATED_AT},
                    ev.{COL_UPDATED_AT},
                    ev.{COL_DELETED_AT}
                FROM {TABLE} ev
                LEFT JOIN element el on el.rowid = ev.{COL_ELEMENT_ID}
                WHERE ev.{COL_UPDATED_AT} > :updated_since
                ORDER BY ev.{COL_UPDATED_AT}, ev.{COL_ROWID}
                LIMIT :limit
            "#
        );
        debug!(query);
        Ok(conn
            .prepare(&query)?
            .query_map(
                named_params! {
                    ":updated_since": updated_since.format(&Rfc3339)?,
                    ":limit": limit.unwrap_or(i64::MAX),
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn select_created_between(
        period_start: &OffsetDateTime,
        period_end: &OffsetDateTime,
        conn: &Connection,
    ) -> Result<Vec<Event>> {
        let start = Instant::now();
        let query = format!(
            r#"
                SELECT
                    ev.{COL_ROWID},
                    ev.{COL_USER_ID},
                    ev.{COL_ELEMENT_ID},
                    json_extract(el.overpass_data, '$.type'),
                    json_extract(el.overpass_data, '$.id'),
                    ev.{COL_TYPE},
                    ev.{COL_TAGS},
                    ev.{COL_CREATED_AT},
                    ev.{COL_UPDATED_AT},
                    ev.{COL_DELETED_AT}
                FROM {TABLE} ev
                LEFT JOIN element el on el.rowid = ev.{COL_ELEMENT_ID}
                WHERE ev.{COL_CREATED_AT} > :period_start AND ev.{COL_CREATED_AT} < :period_end
                ORDER BY ev.{COL_UPDATED_AT}, ev.{COL_ROWID}
            "#
        );
        debug!(query);
        let res = conn
            .prepare(&query)?
            .query_map(
                named_params! {
                    ":period_start": period_start.format(&Rfc3339)?,
                    ":period_end": period_end.format(&Rfc3339)?,
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let time_ms = start.elapsed().as_millis();
        info!(
            count = res.len(),
            time_ms,
            "Loaded {} events in {} ms",
            res.len(),
            time_ms,
        );
        Ok(res)
    }

    pub fn select_by_user(id: i64, limit: i64, conn: &Connection) -> Result<Vec<Event>> {
        let query = format!(
            r#"
                SELECT
                    ev.{COL_ROWID},
                    ev.{COL_USER_ID},
                    ev.{COL_ELEMENT_ID},
                    json_extract(el.overpass_data, '$.type'),
                    json_extract(el.overpass_data, '$.id'),
                    ev.{COL_TYPE},
                    ev.{COL_TAGS},
                    ev.{COL_CREATED_AT},
                    ev.{COL_UPDATED_AT},
                    ev.{COL_DELETED_AT}
                FROM {TABLE} ev
                LEFT JOIN element el on el.rowid = ev.{COL_ELEMENT_ID}
                WHERE ev.{COL_USER_ID} = :id
                ORDER BY ev.{COL_CREATED_AT} DESC
                LIMIT :limit
            "#
        );
        let res = conn
            .prepare(&query)?
            .query_map(named_params! {":id": id, ":limit": limit }, mapper())?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(res)
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<Event>> {
        let query = format!(
            r#"
                SELECT
                    ev.{COL_ROWID},
                    ev.{COL_USER_ID},
                    ev.{COL_ELEMENT_ID},
                    json_extract(el.overpass_data, '$.type'),
                    json_extract(el.overpass_data, '$.id'),
                    ev.{COL_TYPE},
                    ev.{COL_TAGS},
                    ev.{COL_CREATED_AT},
                    ev.{COL_UPDATED_AT},
                    ev.{COL_DELETED_AT}
                FROM {TABLE} ev
                LEFT JOIN element el on el.rowid = ev.{COL_ELEMENT_ID}
                WHERE ev.{COL_ROWID} = :id
            "#
        );
        debug!(query);
        Ok(conn
            .query_row(&query, named_params! { ":id": id }, mapper())
            .optional()?)
    }

    pub fn patch_tags(&self, tags: &HashMap<String, Value>, conn: &Connection) -> Result<Event> {
        Event::_patch_tags(self.id, tags, conn)
    }

    pub fn _patch_tags(id: i64, tags: &HashMap<String, Value>, conn: &Connection) -> Result<Event> {
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_TAGS} = json_patch({COL_TAGS}, :tags)
                WHERE {COL_ROWID} = :id
            "#
        );
        debug!(query);
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            &query,
            named_params! {
                ":id": id,
                ":tags": &serde_json::to_string(tags)?,
            },
        )?;
        Event::select_by_id(id, conn)?.ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    #[cfg(test)]
    pub fn set_updated_at(
        id: i64,
        updated_at: &OffsetDateTime,
        conn: &Connection,
    ) -> Result<Event> {
        let query = format!(
            r#"
                UPDATE {TABLE}
                SET {COL_UPDATED_AT} = :updated_at
                WHERE {COL_ROWID} = :id
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
        Event::select_by_id(id, conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    #[cfg(test)]
    pub fn tag(&self, name: &str) -> &Value {
        self.tags.get(name).unwrap_or(&Value::Null)
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<Event> {
    |row: &Row| -> rusqlite::Result<Event> {
        let tags: String = row.get(6)?;

        Ok(Event {
            id: row.get(0)?,
            user_id: row.get(1)?,
            element_id: row.get(2)?,
            element_osm_type: row.get(3)?,
            element_osm_id: row.get(4)?,
            r#type: row.get(5)?,
            tags: serde_json::from_str(&tags).unwrap(),
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
            deleted_at: row.get(9)?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::Event;
    use crate::{
        element::Element,
        osm::{api::OsmUser, overpass::OverpassElement},
        test::mock_conn,
        user::User,
        Result,
    };
    use serde_json::json;
    use std::collections::HashMap;
    use time::{macros::datetime, OffsetDateTime};

    #[test]
    fn insert() -> Result<()> {
        let conn = mock_conn();
        let user = User::insert(1, &OsmUser::mock(), &conn)?;
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let event = Event::insert(user.id, element.id, "create", &conn)?;
        assert_eq!(event, Event::select_by_id(event.id, &conn)?.unwrap());
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = mock_conn();
        let user = User::insert(1, &OsmUser::mock(), &conn)?;
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        assert_eq!(
            vec![
                Event::insert(user.id, element.id, "", &conn)?,
                Event::insert(user.id, element.id, "", &conn)?,
                Event::insert(user.id, element.id, "", &conn)?,
            ],
            Event::select_all(None, None, &conn)?
        );
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = mock_conn();
        let user = User::insert(1, &OsmUser::mock(), &conn)?;
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let event_1 = Event::insert(user.id, element.id, "", &conn)?;
        let _event_1 = Event::set_updated_at(event_1.id, &datetime!(2020-01-01 00:00 UTC), &conn)?;
        let event_2 = Event::insert(1, element.id, "", &conn)?;
        let event_2 = Event::set_updated_at(event_2.id, &datetime!(2020-01-02 00:00 UTC), &conn)?;
        let event_3 = Event::insert(1, element.id, "", &conn)?;
        let event_3 = Event::set_updated_at(event_3.id, &datetime!(2020-01-03 00:00 UTC), &conn)?;
        assert_eq!(
            vec![event_2, event_3,],
            Event::select_updated_since(&datetime!(2020-01-01 00:00 UTC), None, &conn,)?
        );
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        let user = User::insert(1, &OsmUser::mock(), &conn)?;
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let event = Event::insert(user.id, element.id, "", &conn)?;
        assert_eq!(event, Event::select_by_id(1, &conn)?.unwrap());
        Ok(())
    }

    #[test]
    fn patch_tags() -> Result<()> {
        let conn = mock_conn();
        let tag_1_name = "tag_1_name";
        let tag_1_value_1 = json!("tag_1_value_1");
        let tag_1_value_2 = json!("tag_1_value_2");
        let tag_2_name = "tag_2_name";
        let tag_2_value = json!("tag_2_value");
        let user = User::insert(1, &OsmUser::mock(), &conn)?;
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let event = Event::insert(user.id, element.id, "", &conn)?;
        let mut tags = HashMap::new();
        tags.insert(tag_1_name.into(), tag_1_value_1.clone());
        let event = event.patch_tags(&tags, &conn)?;
        assert_eq!(&tag_1_value_1, event.tag(tag_1_name));
        tags.insert(tag_1_name.into(), tag_1_value_2.clone());
        let event = event.patch_tags(&tags, &conn)?;
        assert_eq!(&tag_1_value_2, event.tag(tag_1_name));
        tags.clear();
        tags.insert(tag_2_name.into(), tag_2_value.clone());
        let event = event.patch_tags(&tags, &conn)?;
        assert!(event.tags.contains_key(tag_1_name));
        assert_eq!(&tag_2_value, event.tag(tag_2_name));
        Ok(())
    }

    #[test]
    fn set_updated_at() -> Result<()> {
        let conn = mock_conn();
        let updated_at = OffsetDateTime::now_utc();
        let user = User::insert(1, &OsmUser::mock(), &conn)?;
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        let event = Event::insert(user.id, element.id, "", &conn)?;
        let event = Event::set_updated_at(event.id, &updated_at, &conn)?;
        assert_eq!(
            updated_at,
            Event::select_by_id(event.id, &conn)?.unwrap().updated_at,
        );
        Ok(())
    }
}
