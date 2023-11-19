use crate::Error;
use crate::Result;
use deadpool_sqlite::Pool;
use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Row;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::debug;

pub struct EventRepo {
    pool: Arc<Pool>,
}

#[derive(PartialEq, Debug)]
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

impl EventRepo {
    pub fn new(pool: &Arc<Pool>) -> Self {
        Self { pool: pool.clone() }
    }

    #[cfg(test)]
    pub async fn insert(&self, user_id: i64, element_id: i64, r#type: &str) -> Result<Event> {
        let r#type = r#type.to_string();
        self.pool
            .get()
            .await?
            .interact(move |conn| Event::insert(user_id, element_id, &r#type, conn))
            .await?
    }

    #[cfg(test)]
    pub async fn _select_all(&self, limit: Option<i64>) -> Result<Vec<Event>> {
        self.pool
            .get()
            .await?
            .interact(move |conn| Event::select_all(limit, conn))
            .await?
    }

    pub async fn select_updated_since(
        &self,
        updated_since: &OffsetDateTime,
        limit: Option<i64>,
    ) -> Result<Vec<Event>> {
        let updated_since = updated_since.clone();
        self.pool
            .get()
            .await?
            .interact(move |conn| Event::select_updated_since(&updated_since, limit, conn))
            .await?
    }

    pub async fn select_by_id(&self, id: i64) -> Result<Option<Event>> {
        self.pool
            .get()
            .await?
            .interact(move |conn| Event::select_by_id(id, conn))
            .await?
    }

    pub async fn _patch_tags(&self, id: i64, tags: &HashMap<String, Value>) -> Result<Event> {
        let tags = tags.clone();
        self.pool
            .get()
            .await?
            .interact(move |conn| Event::_patch_tags(id, &tags, conn))
            .await?
    }

    #[cfg(test)]
    pub async fn set_updated_at(&self, id: i64, updated_at: &OffsetDateTime) -> Result<Event> {
        let updated_at = updated_at.clone();
        self.pool
            .get()
            .await?
            .interact(move |conn| Event::_set_updated_at(id, &updated_at, conn))
            .await?
    }
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
        conn.execute(
            &query,
            named_params! {
                ":user_id": user_id,
                ":element_id": element_id,
                ":type": r#type,
            },
        )?;
        Ok(Event::select_by_id(conn.last_insert_rowid(), &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    #[cfg(test)]
    pub fn select_all(limit: Option<i64>, conn: &Connection) -> Result<Vec<Event>> {
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
                ORDER BY ev.{COL_UPDATED_AT}, ev.{COL_ROWID}
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

    #[cfg(test)]
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
        conn.execute(
            &query,
            named_params! {
                ":id": id,
                ":tags": &serde_json::to_string(tags)?,
            },
        )?;
        Ok(Event::select_by_id(id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
    }

    #[cfg(test)]
    pub fn set_updated_at(&self, updated_at: &OffsetDateTime, conn: &Connection) -> Result<Event> {
        Event::_set_updated_at(self.id, updated_at, conn)
    }

    #[cfg(test)]
    pub fn _set_updated_at(
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
        conn.execute(
            &query,
            named_params! {
                ":id": id,
                ":updated_at": updated_at.format(&Rfc3339)?,
            },
        )?;
        Ok(Event::select_by_id(id, &conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))?)
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
        osm::{osm::OsmUser, overpass::OverpassElement},
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
            Event::select_all(None, &conn)?
        );
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = mock_conn();
        let user = User::insert(1, &OsmUser::mock(), &conn)?;
        let element = Element::insert(&OverpassElement::mock(1), &conn)?;
        Event::insert(user.id, element.id, "", &conn)?
            .set_updated_at(&datetime!(2020-01-01 00:00 UTC), &conn)?;
        assert_eq!(
            vec![
                Event::insert(1, element.id, "", &conn)?
                    .set_updated_at(&datetime!(2020-01-02 00:00 UTC), &conn)?,
                Event::insert(1, element.id, "", &conn)?
                    .set_updated_at(&datetime!(2020-01-03 00:00 UTC), &conn)?,
            ],
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
        let event =
            Event::insert(user.id, element.id, "", &conn)?.set_updated_at(&updated_at, &conn)?;
        assert_eq!(
            updated_at,
            Event::select_by_id(event.id, &conn)?.unwrap().updated_at,
        );
        Ok(())
    }
}
