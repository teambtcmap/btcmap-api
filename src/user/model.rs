use crate::{osm::api::EditingApiUser, Error, Result};
use deadpool_sqlite::Pool;
use rusqlite::{named_params, params, Connection, OptionalExtension, Row};
use serde_json::{Map, Value};
use std::collections::HashMap;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub struct OsmUser {
    pub id: i64,
    pub osm_data: EditingApiUser,
    pub tags: Map<String, Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub deleted_at: Option<OffsetDateTime>,
}

pub struct SelectMostActive {
    pub id: i64,
    pub name: String,
    pub image_url: Option<String>,
    pub description: String,
    pub edits: i64,
    pub created: i64,
    pub updated: i64,
    pub deleted: i64,
}

const TABLE_NAME: &str = "user";

impl OsmUser {
    pub async fn insert_async(id: i64, osm_data: EditingApiUser, pool: &Pool) -> Result<Self> {
        pool.get()
            .await?
            .interact(move |conn| Self::insert(id, &osm_data, conn))
            .await?
    }

    pub fn insert(id: i64, osm_data: &EditingApiUser, conn: &Connection) -> Result<Self> {
        let sql = r#"
            INSERT INTO user (
                rowid,
                osm_data
            ) VALUES (
                :id,
                :osm_data
            )
        "#;
        conn.execute(
            sql,
            named_params! {
                ":id": id,
                ":osm_data": serde_json::to_string(osm_data)?,
            },
        )?;
        Self::select_by_id(conn.last_insert_rowid(), conn)?
            .ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    pub fn select_all(limit: Option<i64>, conn: &Connection) -> Result<Vec<Self>> {
        let sql = r#"
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
            .prepare(sql)?
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
    ) -> Result<Vec<Self>> {
        let sql = r#"
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
            .prepare(sql)?
            .query_map(
                named_params! {
                    ":updated_since": updated_since.format(&Rfc3339)?,
                    ":limit": limit.unwrap_or(i64::MAX)
                },
                mapper(),
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub async fn select_most_active_async(
        period_start: OffsetDateTime,
        period_end: OffsetDateTime,
        limit: i64,
        pool: &Pool,
    ) -> Result<Vec<SelectMostActive>> {
        pool.get()
            .await?
            .interact(move |conn| Self::select_most_active(period_start, period_end, limit, conn))
            .await?
    }

    pub fn select_most_active(
        period_start: OffsetDateTime,
        period_end: OffsetDateTime,
        limit: i64,
        conn: &Connection,
    ) -> Result<Vec<SelectMostActive>> {
        let sql = r#"
            select 
                u.id,
                json_extract(u.osm_data, '$.display_name') as name,
                json_extract(u.osm_data, '$.img.href') as image_url,
                json_extract(u.osm_data, '$.description') as description,
                count(*) as edits,
                (select count(*) from event where user_id = u.id and created_at between ?1 and ?2 and type = 'create') as created,
                (select count(*) from event where user_id = u.id and created_at between ?1 and ?2 and type = 'update') as updated,
                (select count(*) from event where user_id = u.id and created_at between ?1 and ?2 and type = 'delete') as deleted
            from event e join user u on u.id = e.user_id where e.created_at between ?1 and ?2
            group by e.user_id
            order by edits desc
            limit ?3
        "#;
        conn.prepare(sql)?
            .query_map(
                params![
                    period_start.format(&Rfc3339)?,
                    period_end.format(&Rfc3339)?,
                    limit,
                ],
                mapper_select_ordered_by_severity(),
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn select_by_id_or_name(id_or_name: &str, conn: &Connection) -> Result<Option<Self>> {
        match id_or_name.parse::<i64>() {
            Ok(id) => Self::select_by_id(id, conn),
            Err(_) => Self::select_by_name(id_or_name, conn),
        }
    }

    pub async fn select_by_id_async(id: i64, pool: &Pool) -> Result<Option<Self>> {
        pool.get()
            .await?
            .interact(move |conn| Self::select_by_id(id, conn))
            .await?
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<Self>> {
        let sql = r#"
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
            .query_row(sql, named_params! { ":id": id }, mapper())
            .optional()?)
    }

    pub fn select_by_name(name: &str, conn: &Connection) -> Result<Option<Self>> {
        let sql = format!(
            r#"
                SELECT                 
                    rowid,
                    osm_data,
                    tags,
                    created_at,
                    updated_at,
                    deleted_at
                FROM {TABLE_NAME}
                WHERE json_extract(osm_data, '$.display_name') = :name
            "#
        );
        let res = conn
            .query_row(&sql, named_params! { ":name": name }, mapper())
            .optional()?;
        Ok(res)
    }

    pub async fn set_tag_async(id: i64, name: String, value: Value, pool: &Pool) -> Result<Self> {
        pool.get()
            .await?
            .interact(move |conn| Self::set_tag(id, &name, &value, conn))
            .await?
    }

    pub fn set_tag(id: i64, name: &str, value: &Value, conn: &Connection) -> Result<Self> {
        let mut patch_set = HashMap::new();
        patch_set.insert(name.into(), value.clone());
        Self::patch_tags(id, &patch_set, conn)
    }

    pub fn patch_tags(
        id: i64,
        tags: &HashMap<String, Value>,
        conn: &Connection,
    ) -> crate::Result<Self> {
        let sql = format!(
            r#"
                UPDATE {TABLE_NAME}
                SET tags = json_patch(tags, :tags)
                WHERE rowid = :id
            "#
        );
        conn.execute(
            &sql,
            named_params! { ":id": id, ":tags": &serde_json::to_string(tags)? },
        )?;
        Self::select_by_id(id, conn)?.ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    pub fn remove_tag(id: i64, name: &str, conn: &Connection) -> Result<Option<Self>> {
        let sql = format!(
            r#"
                UPDATE {TABLE_NAME}
                SET tags = json_remove(tags, :name)
                WHERE id = :id
            "#
        );
        conn.execute(
            &sql,
            named_params! {
                ":id": id,
                ":name": format!("$.{name}"),
            },
        )?;
        let res = Self::select_by_id(id, conn)?;
        Ok(res)
    }

    pub async fn set_osm_data_async(id: i64, osm_data: EditingApiUser, pool: &Pool) -> Result<()> {
        pool.get()
            .await?
            .interact(move |conn| Self::set_osm_data(id, &osm_data, conn))
            .await?
    }

    pub fn set_osm_data(id: i64, osm_data: &EditingApiUser, conn: &Connection) -> Result<()> {
        let sql = r#"
            UPDATE user
            SET osm_data = json(:osm_data)
            WHERE rowid = :id
        "#;
        conn.execute(
            sql,
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
    ) -> Result<Self> {
        let sql = r#"
                UPDATE user
                SET updated_at = :updated_at
                WHERE rowid = :id
            "#
        .to_string();
        conn.execute(
            &sql,
            named_params! {
                ":id": id,
                ":updated_at": updated_at.format(&Rfc3339)?,
            },
        )?;
        Self::select_by_id(id, conn)?.ok_or(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows))
    }
}

const fn mapper() -> fn(&Row) -> rusqlite::Result<OsmUser> {
    |row: &Row| -> rusqlite::Result<OsmUser> {
        let osm_data: String = row.get(1)?;
        let tags: String = row.get(2)?;

        Ok(OsmUser {
            id: row.get(0)?,
            osm_data: serde_json::from_str(&osm_data).unwrap(),
            tags: serde_json::from_str(&tags).unwrap(),
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
        })
    }
}

const fn mapper_select_ordered_by_severity() -> fn(&Row) -> rusqlite::Result<SelectMostActive> {
    |row: &Row| -> rusqlite::Result<SelectMostActive> {
        Ok(SelectMostActive {
            id: row.get(0)?,
            name: row.get(1)?,
            image_url: row.get(2)?,
            description: row.get(3)?,
            edits: row.get(4)?,
            created: row.get(5)?,
            updated: row.get(6)?,
            deleted: row.get(7)?,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{osm::api::EditingApiUser, test::mock_conn, user::OsmUser, Result};
    use std::collections::HashMap;
    use time::macros::datetime;

    #[test]
    fn insert() -> Result<()> {
        let conn = mock_conn();
        OsmUser::insert(1, &EditingApiUser::mock(), &conn)?;
        let users = OsmUser::select_all(None, &conn)?;
        assert_eq!(1, users.len());
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = mock_conn();
        OsmUser::insert(1, &EditingApiUser::mock(), &conn)?;
        OsmUser::insert(2, &EditingApiUser::mock(), &conn)?;
        OsmUser::insert(3, &EditingApiUser::mock(), &conn)?;
        let reports = OsmUser::select_all(None, &conn)?;
        assert_eq!(3, reports.len());
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = mock_conn();
        conn.execute(
            "INSERT INTO user (rowid, osm_data, updated_at) VALUES (1, json(?), '2020-01-01T00:00:00Z')",
            [serde_json::to_string(&EditingApiUser::mock())?],
        )?;
        conn.execute(
            "INSERT INTO user (rowid, osm_data, updated_at) VALUES (2, json(?), '2020-01-02T00:00:00Z')",
            [serde_json::to_string(&EditingApiUser::mock())?],
        )?;
        conn.execute(
            "INSERT INTO user (rowid, osm_data, updated_at) VALUES (3, json(?), '2020-01-03T00:00:00Z')",
            [serde_json::to_string(&EditingApiUser::mock())?],
        )?;
        assert_eq!(
            2,
            OsmUser::select_updated_since(&datetime!(2020-01-01 00:00:00 UTC), None, &conn)?.len()
        );
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        OsmUser::insert(1, &EditingApiUser::mock(), &conn)?;
        assert!(OsmUser::select_by_id(1, &conn)?.is_some());
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
        OsmUser::insert(1, &EditingApiUser::mock(), &conn)?;
        let user = OsmUser::select_by_id(1, &conn)?.unwrap();
        assert!(user.tags.is_empty());
        OsmUser::patch_tags(1, &tags, &conn)?;
        let user = OsmUser::select_by_id(1, &conn)?.unwrap();
        assert_eq!(1, user.tags.len());
        tags.insert(tag_2_name.into(), tag_2_value.into());
        OsmUser::patch_tags(1, &tags, &conn)?;
        let user = OsmUser::select_by_id(1, &conn)?.unwrap();
        assert_eq!(2, user.tags.len());
        Ok(())
    }

    #[test]
    fn set_osm_data() -> Result<()> {
        let conn = mock_conn();
        let user = EditingApiUser {
            id: 1,
            ..EditingApiUser::mock()
        };
        OsmUser::insert(user.id, &user, &conn)?;
        let user = EditingApiUser {
            id: 2,
            ..EditingApiUser::mock()
        };
        OsmUser::set_osm_data(1, &user, &conn)?;
        let user = OsmUser::select_by_id(1, &conn)?.unwrap();
        assert_eq!(2, user.osm_data.id);
        Ok(())
    }
}
