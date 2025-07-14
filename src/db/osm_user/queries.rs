use super::schema;
use super::schema::Columns;
use crate::{service::osm::EditingApiUser, Result};
use rusqlite::{params, Connection, Row};
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

impl OsmUser {
    fn projection() -> String {
        [
            Columns::Id,
            Columns::OsmData,
            Columns::Tags,
            Columns::CreatedAt,
            Columns::UpdatedAt,
            Columns::DeletedAt,
        ]
        .iter()
        .map(Columns::as_str)
        .collect::<Vec<_>>()
        .join(", ")
    }

    fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row: &Row| -> rusqlite::Result<Self> {
            let osm_data: String = row.get(1)?;
            let tags: String = row.get(2)?;

            Ok(Self {
                id: row.get(0)?,
                osm_data: serde_json::from_str(&osm_data).unwrap(),
                tags: serde_json::from_str(&tags).unwrap(),
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                deleted_at: row.get(5)?,
            })
        }
    }
}

pub fn insert(id: i64, osm_data: &EditingApiUser, conn: &Connection) -> Result<OsmUser> {
    let sql = format!(
        r#"
            INSERT INTO {table} (
                {id},
                {osm_data}
            ) VALUES (
                ?1,
                ?2
            )
        "#,
        id = Columns::Id.as_str(),
        osm_data = Columns::OsmData.as_str(),
        table = schema::NAME
    );
    conn.execute(&sql, params![id, serde_json::to_string(osm_data)?])?;
    select_by_id(conn.last_insert_rowid(), conn)
}

pub fn select_all(limit: Option<i64>, conn: &Connection) -> Result<Vec<OsmUser>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            ORDER BY {updated_at}, {id}
            LIMIT ?1
        "#,
        projection = OsmUser::projection(),
        table = schema::NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    Ok(conn
        .prepare(&sql)?
        .query_map(params![limit.unwrap_or(i64::MAX)], OsmUser::mapper())?
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn select_updated_since(
    updated_since: &OffsetDateTime,
    limit: Option<i64>,
    conn: &Connection,
) -> Result<Vec<OsmUser>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {updated_at} > ?1
            ORDER BY {updated_at}, {id}
            LIMIT ?2
        "#,
        projection = OsmUser::projection(),
        table = schema::NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    Ok(conn
        .prepare(&sql)?
        .query_map(
            params![updated_since.format(&Rfc3339)?, limit.unwrap_or(i64::MAX)],
            OsmUser::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()?)
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

impl SelectMostActive {
    fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row: &Row| -> rusqlite::Result<Self> {
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
}

pub fn select_most_active(
    period_start: OffsetDateTime,
    period_end: OffsetDateTime,
    limit: i64,
    conn: &Connection,
) -> Result<Vec<SelectMostActive>> {
    let sql = format!(
        r#"
            SELECT 
                u.{u_id},
                json_extract(u.{u_osm_data}, '$.display_name'),
                json_extract(u.{u_osm_data}, '$.img.href'),
                json_extract(u.{u_osm_data}, '$.description'),
                count(*) AS edits,
                (SELECT count(*) FROM event WHERE user_id = u.id AND created_at between ?1 AND ?2 AND type = 'create'),
                (SELECT count(*) FROM event WHERE user_id = u.id AND created_at between ?1 AND ?2 AND type = 'update'),
                (SELECT count(*) FROM event WHERE user_id = u.id AND created_at between ?1 AND ?2 AND type = 'delete')
            FROM event e JOIN {table} u ON u.{u_id} = e.user_id WHERE e.created_at BETWEEN ?1 AND ?2
            GROUP BY e.user_id
            ORDER BY edits DESC
            LIMIT ?3
        "#,
        u_id = Columns::Id.as_str(),
        u_osm_data = Columns::OsmData.as_str(),
        table = schema::NAME,
    );
    conn.prepare(&sql)?
        .query_map(
            params![
                period_start.format(&Rfc3339)?,
                period_end.format(&Rfc3339)?,
                limit,
            ],
            SelectMostActive::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id_or_name(id_or_name: &str, conn: &Connection) -> Result<OsmUser> {
    match id_or_name.parse::<i64>() {
        Ok(id) => select_by_id(id, conn),
        Err(_) => select_by_name(id_or_name, conn),
    }
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<OsmUser> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = OsmUser::projection(),
        table = schema::NAME,
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![id], OsmUser::mapper())
        .map_err(Into::into)
}

pub fn select_by_name(name: &str, conn: &Connection) -> Result<OsmUser> {
    let sql = format!(
        r#"
                SELECT {projection}
                FROM {table}
                WHERE json_extract({osm_data}, '$.display_name') = ?1
        "#,
        projection = OsmUser::projection(),
        table = schema::NAME,
        osm_data = Columns::OsmData.as_str(),
    );
    conn.query_row(&sql, params![name], OsmUser::mapper())
        .map_err(Into::into)
}

pub fn set_osm_data(id: i64, osm_data: &EditingApiUser, conn: &Connection) -> Result<()> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {osm_data} = json(?1)
            WHERE {id} = ?2
        "#,
        table = schema::NAME,
        osm_data = Columns::OsmData.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![serde_json::to_string(osm_data)?, id,])?;
    Ok(())
}

pub fn set_tag(id: i64, name: &str, value: &Value, conn: &Connection) -> Result<OsmUser> {
    let mut patch_set = HashMap::new();
    patch_set.insert(name.into(), value.clone());
    patch_tags(id, &patch_set, conn)
}

pub fn patch_tags(id: i64, tags: &HashMap<String, Value>, conn: &Connection) -> Result<OsmUser> {
    let sql = format!(
        r#"
                UPDATE {table}
                SET {tags} = json_patch({tags}, ?1)
                WHERE {id} = ?2
        "#,
        table = schema::NAME,
        tags = Columns::Tags.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![&serde_json::to_string(tags)?, id])?;
    select_by_id(id, conn)
}

pub fn remove_tag(id: i64, name: &str, conn: &Connection) -> Result<OsmUser> {
    let sql = format!(
        r#"
                UPDATE {table}
                SET {tags} = json_remove({tags}, ?1)
                WHERE {id} = ?2
        "#,
        table = schema::NAME,
        tags = Columns::Tags.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![format!("$.{name}"), id,])?;
    select_by_id(id, conn)
}

#[cfg(test)]
pub fn set_updated_at(id: i64, updated_at: OffsetDateTime, conn: &Connection) -> Result<OsmUser> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {updated_at} = ?1
            WHERE {id} = ?2
        "#,
        table = schema::NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![updated_at.format(&Rfc3339)?, id,])?;
    select_by_id(id, conn)
}

#[cfg(test)]
mod test {
    use crate::{
        db::{self, test::conn},
        service::{
            osm::{Blocks, BlocksReceived, Changesets, ContributorTerms, EditingApiUser, Traces},
            overpass::OverpassElement,
        },
        Result,
    };
    use serde_json::Value;
    use std::collections::HashMap;
    use time::{macros::datetime, Duration, OffsetDateTime};

    #[test]
    fn insert() -> Result<()> {
        let conn = conn();
        super::insert(1, &EditingApiUser::mock(), &conn)?;
        let users = super::select_all(None, &conn)?;
        assert_eq!(1, users.len());
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = conn();
        super::insert(1, &EditingApiUser::mock(), &conn)?;
        super::insert(2, &EditingApiUser::mock(), &conn)?;
        super::insert(3, &EditingApiUser::mock(), &conn)?;
        let reports = super::select_all(None, &conn)?;
        assert_eq!(3, reports.len());
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = conn();
        let _u1 = super::insert(1, &EditingApiUser::mock(), &conn)?;
        let _u1 = super::set_updated_at(_u1.id, datetime!(2020-01-01 00:00:00 UTC), &conn)?;
        let _u2 = super::insert(2, &EditingApiUser::mock(), &conn)?;
        let _u2 = super::set_updated_at(_u2.id, datetime!(2020-01-02 00:00:00 UTC), &conn)?;
        let _u3 = super::insert(3, &EditingApiUser::mock(), &conn)?;
        let _u3 = super::set_updated_at(_u3.id, datetime!(2020-01-03 00:00:00 UTC), &conn)?;
        assert_eq!(
            2,
            super::select_updated_since(&datetime!(2020-01-01 00:00:00 UTC), None, &conn)?.len()
        );
        Ok(())
    }

    #[test]
    fn select_most_active() -> Result<()> {
        let conn = conn();
        let res = super::select_most_active(
            OffsetDateTime::now_utc(),
            OffsetDateTime::now_utc(),
            10,
            &conn,
        )?;
        assert_eq!(0, res.len());
        let user = super::insert(1, &EditingApiUser::mock(), &conn)?;
        let element = db::element::queries::insert(&OverpassElement::mock(1), &conn)?;
        let _event_1 = db::event::queries::insert(user.id, element.id, "update", &conn)?;
        let _event_2 = db::event::queries::insert(user.id, element.id, "update", &conn)?;
        let _event_3 = db::event::queries::insert(user.id, element.id, "update", &conn)?;
        let res = super::select_most_active(
            OffsetDateTime::now_utc().saturating_add(Duration::days(-1)),
            OffsetDateTime::now_utc().saturating_add(Duration::days(1)),
            10,
            &conn,
        )?;
        assert_eq!(1, res.len());
        assert_eq!(3, res.first().unwrap().updated);
        assert_eq!(3, res.first().unwrap().edits);
        Ok(())
    }

    #[test]
    fn select_by_id_or_name() -> Result<()> {
        let conn = conn();
        let user = super::insert(1, &EditingApiUser::mock(), &conn)?;
        assert_eq!(user.id, super::select_by_id(1, &conn)?.id);
        assert_eq!(
            user.osm_data.display_name,
            super::select_by_name(&user.osm_data.display_name, &conn)?
                .osm_data
                .display_name
        );
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = conn();
        super::insert(1, &EditingApiUser::mock(), &conn)?;
        assert!(super::select_by_id(1, &conn).is_ok());
        Ok(())
    }

    #[test]
    fn select_by_name() -> Result<()> {
        let name = "";
        let conn = conn();
        let _user = EditingApiUser {
            id: 1,
            display_name: name.into(),
            account_created: OffsetDateTime::now_utc(),
            description: "".into(),
            contributor_terms: ContributorTerms { agreed: true },
            img: None,
            roles: vec![],
            changesets: Changesets { count: 0 },
            traces: Traces { count: 0 },
            blocks: Blocks {
                received: BlocksReceived {
                    count: 0,
                    active: 0,
                },
            },
        };
        let _user = super::insert(1, &EditingApiUser::mock(), &conn)?;
        assert_eq!(
            name,
            super::select_by_name(name, &conn)?.osm_data.display_name
        );
        Ok(())
    }

    #[test]
    fn set_osm_data() -> Result<()> {
        let conn = conn();
        let user = EditingApiUser {
            id: 1,
            ..EditingApiUser::mock()
        };
        super::insert(user.id, &user, &conn)?;
        let user = EditingApiUser {
            id: 2,
            ..EditingApiUser::mock()
        };
        super::set_osm_data(1, &user, &conn)?;
        let user = super::select_by_id(1, &conn)?;
        assert_eq!(2, user.osm_data.id);
        Ok(())
    }

    #[test]
    fn set_tag() -> Result<()> {
        let tag_name = "foo";
        let tag_value = Value::String("bar".into());
        let conn = conn();
        let user = super::insert(1, &EditingApiUser::mock(), &conn)?;
        let user = super::set_tag(user.id, tag_name, &tag_value, &conn)?;
        assert_eq!(tag_value, user.tags[tag_name]);
        Ok(())
    }

    #[test]
    fn patch_tags() -> Result<()> {
        let conn = conn();
        let tag_1_name = "foo";
        let tag_1_value = "bar";
        let tag_2_name = "qwerty";
        let tag_2_value = "test";
        let mut tags = HashMap::new();
        tags.insert(tag_1_name.into(), tag_1_value.into());
        super::insert(1, &EditingApiUser::mock(), &conn)?;
        let user = super::select_by_id(1, &conn)?;
        assert!(user.tags.is_empty());
        super::patch_tags(1, &tags, &conn)?;
        let user = super::select_by_id(1, &conn)?;
        assert_eq!(1, user.tags.len());
        tags.insert(tag_2_name.into(), tag_2_value.into());
        super::patch_tags(1, &tags, &conn)?;
        let user = super::select_by_id(1, &conn)?;
        assert_eq!(2, user.tags.len());
        Ok(())
    }

    #[test]
    fn remove_tag() -> Result<()> {
        let tag_name = "foo";
        let tag_value = Value::String("bar".into());
        let conn = conn();
        let user = super::insert(1, &EditingApiUser::mock(), &conn)?;
        let user = super::set_tag(user.id, tag_name, &tag_value, &conn)?;
        assert_eq!(tag_value, user.tags[tag_name]);
        let user = super::remove_tag(user.id, tag_name, &conn)?;
        assert!(user.tags.get(tag_name).is_none());
        Ok(())
    }

    #[test]
    fn set_updated_at() -> Result<()> {
        let updated_at = OffsetDateTime::now_utc().saturating_add(Duration::hours(1));
        let conn = conn();
        let user = super::insert(1, &EditingApiUser::mock(), &conn)?;
        let user = super::set_updated_at(user.id, updated_at, &conn)?;
        assert_eq!(updated_at, user.updated_at);
        Ok(())
    }
}
