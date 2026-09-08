use super::schema::{self, Columns, ElectrumServer};
use crate::Result;
use rusqlite::{named_params, params, Connection, ToSql};
use schema::Columns::*;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn select_all(conn: &Connection) -> Result<Vec<ElectrumServer>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            ORDER BY {priority} DESC, {id} ASC
        "#,
        projection = ElectrumServer::projection(),
        table = schema::TABLE_NAME,
        priority = Columns::Priority.as_ref(),
        id = Columns::Id.as_ref(),
    );
    conn.prepare(&sql)?
        .query_map([], ElectrumServer::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<ElectrumServer> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = ElectrumServer::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_ref(),
    );
    conn.query_row(&sql, params![id], ElectrumServer::mapper())
        .map_err(Into::into)
}

pub fn insert(
    name: &str,
    url: &str,
    priority: i64,
    spki_pin: &str,
    conn: &Connection,
) -> Result<ElectrumServer> {
    let sql = format!(
        r#"
            INSERT INTO {TABLE_NAME} ({Name}, {Url}, {Priority}, {SpkiPin})
            VALUES (:name, :url, :priority, :spki_pin)
            RETURNING {projection}
        "#,
        TABLE_NAME = schema::TABLE_NAME,
        projection = ElectrumServer::projection(),
    );
    let params = named_params! {
        ":name": name,
        ":url": url,
        ":priority": priority,
        ":spki_pin": spki_pin,
    };
    conn.query_row(&sql, params, ElectrumServer::mapper())
        .map_err(Into::into)
}

pub fn update(
    id: i64,
    name: Option<&str>,
    url: Option<&str>,
    priority: Option<i64>,
    spki_pin: Option<&str>,
    conn: &Connection,
) -> Result<ElectrumServer> {
    let mut sets: Vec<String> = Vec::new();
    let mut sql_params: Vec<(&str, &dyn ToSql)> = vec![(":id", &id)];

    if let Some(v) = &name {
        sets.push(format!("{Name} = :name"));
        sql_params.push((":name", v));
    }
    if let Some(v) = &url {
        sets.push(format!("{Url} = :url"));
        sql_params.push((":url", v));
    }
    if let Some(v) = &priority {
        sets.push(format!("{Priority} = :priority"));
        sql_params.push((":priority", v));
    }
    if let Some(v) = &spki_pin {
        sets.push(format!("{SpkiPin} = :spki_pin"));
        sql_params.push((":spki_pin", v));
    }

    if sets.is_empty() {
        return select_by_id(id, conn);
    }

    let sql = format!(
        r#"
            UPDATE {TABLE_NAME}
            SET {}
            WHERE {Id} = :id
            RETURNING {projection}
        "#,
        sets.join(", "),
        TABLE_NAME = schema::TABLE_NAME,
        projection = ElectrumServer::projection(),
    );
    conn.query_row(&sql, sql_params.as_slice(), ElectrumServer::mapper())
        .map_err(Into::into)
}

pub fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    conn: &Connection,
) -> Result<ElectrumServer> {
    match deleted_at {
        Some(deleted_at) => {
            let sql = format!(
                r#"
                    UPDATE {TABLE_NAME}
                    SET {DeletedAt} = ?2
                    WHERE {Id} = ?1
                    RETURNING {projection}
                "#,
                TABLE_NAME = schema::TABLE_NAME,
                projection = ElectrumServer::projection(),
            );
            conn.query_row(
                &sql,
                params![id, deleted_at.format(&Rfc3339)?],
                ElectrumServer::mapper(),
            )
            .map_err(Into::into)
        }
        None => {
            let sql = format!(
                r#"
                    UPDATE {TABLE_NAME}
                    SET {DeletedAt} = NULL
                    WHERE {Id} = ?1
                    RETURNING {projection}
                "#,
                TABLE_NAME = schema::TABLE_NAME,
                projection = ElectrumServer::projection(),
            );
            conn.query_row(&sql, params![id], ElectrumServer::mapper())
                .map_err(Into::into)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{db::main::test::conn, Result};
    use time::OffsetDateTime;

    #[test]
    fn insert_and_select_by_id() -> Result<()> {
        let conn = conn();
        let server = super::insert("foo", "ssl://foo:50002", 10, "", &conn)?;
        assert_eq!(server.name, "foo");
        assert_eq!(server.url, "ssl://foo:50002");
        assert_eq!(server.priority, 10);
        assert_eq!(server.spki_pin, "");
        let fetched = super::select_by_id(server.id, &conn)?;
        assert_eq!(server, fetched);
        Ok(())
    }

    #[test]
    fn insert_with_spki_pin() -> Result<()> {
        let conn = conn();
        let server = super::insert(
            "foo",
            "ssl://foo:50002",
            0,
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            &conn,
        )?;
        assert!(server.spki_pin.starts_with("sha256:"));
        Ok(())
    }

    #[test]
    fn insert_default_priority() -> Result<()> {
        let conn = conn();
        let server = super::insert("foo", "ssl://foo:50002", 0, "", &conn)?;
        assert_eq!(server.priority, 0);
        Ok(())
    }

    #[test]
    fn select_all_orders_by_priority_desc() -> Result<()> {
        let conn = conn();
        let s1 = super::insert("low", "ssl://low:50002", 1, "", &conn)?;
        let s2 = super::insert("high", "ssl://high:50002", 100, "", &conn)?;
        let s3 = super::insert("mid", "ssl://mid:50002", 50, "", &conn)?;
        let all = super::select_all(&conn)?;
        assert_eq!(vec![s2, s3, s1], all);
        Ok(())
    }

    #[test]
    fn update_changes_specified_fields() -> Result<()> {
        let conn = conn();
        let server = super::insert("foo", "ssl://foo:50002", 10, "", &conn)?;
        let updated = super::update(
            server.id,
            Some("bar"),
            Some("ssl://bar:50002"),
            Some(99),
            Some("sha256:1111111111111111111111111111111111111111111111111111111111111111"),
            &conn,
        )?;
        assert_eq!(updated.name, "bar");
        assert_eq!(updated.url, "ssl://bar:50002");
        assert_eq!(updated.priority, 99);
        assert!(updated.spki_pin.starts_with("sha256:11111111"));
        assert!(updated.updated_at >= server.updated_at);
        Ok(())
    }

    #[test]
    fn update_partial_only_name() -> Result<()> {
        let conn = conn();
        let server = super::insert("foo", "ssl://foo:50002", 10, "pin", &conn)?;
        let updated = super::update(server.id, Some("bar"), None, None, None, &conn)?;
        assert_eq!(updated.name, "bar");
        assert_eq!(updated.url, "ssl://foo:50002");
        assert_eq!(updated.priority, 10);
        assert_eq!(updated.spki_pin, "pin");
        Ok(())
    }

    #[test]
    fn update_no_fields_returns_existing() -> Result<()> {
        let conn = conn();
        let server = super::insert("foo", "ssl://foo:50002", 10, "", &conn)?;
        let original_updated_at = server.updated_at;
        let returned = super::update(server.id, None, None, None, None, &conn)?;
        assert_eq!(returned, server);
        assert_eq!(returned.updated_at, original_updated_at);
        Ok(())
    }

    #[test]
    fn update_missing_row() {
        let conn = conn();
        let res = super::update(999, Some("bar"), None, None, None, &conn);
        assert!(res.is_err());
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = conn();
        let server = super::insert("foo", "ssl://foo:50002", 0, "", &conn)?;
        let deleted = super::set_deleted_at(server.id, Some(OffsetDateTime::now_utc()), &conn)?;
        assert!(deleted.deleted_at.is_some());
        let restored = super::set_deleted_at(server.id, None, &conn)?;
        assert!(restored.deleted_at.is_none());
        Ok(())
    }

    #[test]
    fn insert_rejects_duplicate_url() {
        let conn = conn();
        super::insert("foo", "ssl://foo:50002", 0, "", &conn).unwrap();
        let res = super::insert("bar", "ssl://foo:50002", 0, "", &conn);
        assert!(res.is_err());
    }
}
