use super::schema::{self, CachedTx, Columns, Wallet};
use crate::Result;
use rusqlite::{named_params, params, Connection, ToSql};
use schema::Columns::*;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn select_all(conn: &Connection) -> Result<Vec<Wallet>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            ORDER BY {id} ASC
        "#,
        projection = Wallet::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_ref(),
    );
    conn.prepare(&sql)?
        .query_map([], Wallet::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<Wallet> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = Wallet::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_ref(),
    );
    conn.query_row(&sql, params![id], Wallet::mapper())
        .map_err(Into::into)
}

pub fn insert(name: &str, xpub: &str, conn: &Connection) -> Result<Wallet> {
    let sql = format!(
        r#"
            INSERT INTO {TABLE_NAME} ({Name}, {Xpub})
            VALUES (:name, :xpub)
            RETURNING {projection}
        "#,
        TABLE_NAME = schema::TABLE_NAME,
        projection = Wallet::projection(),
    );
    let params = named_params! {
        ":name": name,
        ":xpub": xpub,
    };
    conn.query_row(&sql, params, Wallet::mapper())
        .map_err(Into::into)
}

pub fn update(
    id: i64,
    name: Option<&str>,
    xpub: Option<&str>,
    conn: &Connection,
) -> Result<Wallet> {
    let mut sets: Vec<String> = Vec::new();
    let mut sql_params: Vec<(&str, &dyn ToSql)> = vec![(":id", &id)];

    if let Some(v) = &name {
        sets.push(format!("{Name} = :name"));
        sql_params.push((":name", v));
    }
    if let Some(v) = &xpub {
        sets.push(format!("{Xpub} = :xpub"));
        sql_params.push((":xpub", v));
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
        projection = Wallet::projection(),
    );
    conn.query_row(&sql, sql_params.as_slice(), Wallet::mapper())
        .map_err(Into::into)
}

pub fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    conn: &Connection,
) -> Result<Wallet> {
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
                projection = Wallet::projection(),
            );
            conn.query_row(
                &sql,
                params![id, deleted_at.format(&Rfc3339)?],
                Wallet::mapper(),
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
                projection = Wallet::projection(),
            );
            conn.query_row(&sql, params![id], Wallet::mapper())
                .map_err(Into::into)
        }
    }
}

pub fn set_cached_snapshot(
    id: i64,
    balance_sats: i64,
    tx: &[CachedTx],
    cached_at: OffsetDateTime,
    conn: &Connection,
) -> Result<Wallet> {
    let sql = format!(
        r#"
            UPDATE {TABLE_NAME}
            SET {CachedBalanceSats} = :balance_sats,
                {CachedTx} = :cached_tx,
                {CachedAt} = :cached_at
            WHERE {Id} = :id
            RETURNING {projection}
        "#,
        TABLE_NAME = schema::TABLE_NAME,
        projection = Wallet::projection(),
    );
    let cached_tx = serde_json::to_string(tx)
        .map_err(|e| crate::Error::Other(format!("failed to serialize wallet cached_tx: {}", e)))?;
    let params = named_params! {
        ":id": id,
        ":balance_sats": balance_sats,
        ":cached_tx": cached_tx,
        ":cached_at": cached_at.format(&Rfc3339)?,
    };
    conn.query_row(&sql, params, Wallet::mapper())
        .map_err(Into::into)
}

#[cfg(test)]
mod test {
    use crate::{db::main::test::conn, Result};
    use time::OffsetDateTime;

    #[test]
    fn insert_and_select_by_id() -> Result<()> {
        let conn = conn();
        let wallet = super::insert(
            "foo",
            "xpub0000000000000000000000000000000000000000000000000000000000000000",
            &conn,
        )?;
        assert_eq!(wallet.name, "foo");
        assert_eq!(
            wallet.xpub,
            "xpub0000000000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(wallet.cached_balance_sats, 0);
        assert!(wallet.cached_tx.is_empty());
        assert!(wallet.cached_at.is_none());
        let fetched = super::select_by_id(wallet.id, &conn)?;
        assert_eq!(fetched.name, wallet.name);
        assert_eq!(fetched.xpub, wallet.xpub);
        Ok(())
    }

    #[test]
    fn insert_rejects_duplicate_name() {
        let conn = conn();
        super::insert(
            "foo",
            "xpub0000000000000000000000000000000000000000000000000000000000000000",
            &conn,
        )
        .unwrap();
        let res = super::insert(
            "foo",
            "xpub0000000000000000000000000000000000000000000000000000000000000000",
            &conn,
        );
        assert!(res.is_err());
    }

    #[test]
    fn select_all_orders_by_id_ascending() -> Result<()> {
        let conn = conn();
        let _ = super::insert(
            "a",
            "xpub0000000000000000000000000000000000000000000000000000000000000000",
            &conn,
        )?;
        let _ = super::insert(
            "b",
            "xpub0000000000000000000000000000000000000000000000000000000000000000",
            &conn,
        )?;
        let _ = super::insert(
            "c",
            "xpub0000000000000000000000000000000000000000000000000000000000000000",
            &conn,
        )?;
        let all = super::select_all(&conn)?;
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].name, "a");
        assert_eq!(all[1].name, "b");
        assert_eq!(all[2].name, "c");
        Ok(())
    }

    #[test]
    fn update_changes_specified_fields() -> Result<()> {
        let conn = conn();
        let wallet = super::insert(
            "foo",
            "xpub0000000000000000000000000000000000000000000000000000000000000000",
            &conn,
        )?;
        let updated = super::update(
            wallet.id,
            Some("bar"),
            Some("xpub1111111111111111111111111111111111111111111111111111111111111111"),
            &conn,
        )?;
        assert_eq!(updated.name, "bar");
        assert_eq!(
            updated.xpub,
            "xpub1111111111111111111111111111111111111111111111111111111111111111"
        );
        assert!(updated.updated_at >= wallet.updated_at);
        Ok(())
    }

    #[test]
    fn update_partial_only_name() -> Result<()> {
        let conn = conn();
        let wallet = super::insert(
            "foo",
            "xpub0000000000000000000000000000000000000000000000000000000000000000",
            &conn,
        )?;
        let updated = super::update(wallet.id, Some("bar"), None, &conn)?;
        assert_eq!(updated.name, "bar");
        assert_eq!(
            updated.xpub,
            "xpub0000000000000000000000000000000000000000000000000000000000000000"
        );
        Ok(())
    }

    #[test]
    fn update_no_fields_returns_existing() -> Result<()> {
        let conn = conn();
        let wallet = super::insert(
            "foo",
            "xpub0000000000000000000000000000000000000000000000000000000000000000",
            &conn,
        )?;
        let original_updated_at = wallet.updated_at;
        let returned = super::update(wallet.id, None, None, &conn)?;
        assert_eq!(returned.name, wallet.name);
        assert_eq!(returned.updated_at, original_updated_at);
        Ok(())
    }

    #[test]
    fn update_missing_row() {
        let conn = conn();
        let res = super::update(999, Some("bar"), None, &conn);
        assert!(res.is_err());
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = conn();
        let wallet = super::insert(
            "foo",
            "xpub0000000000000000000000000000000000000000000000000000000000000000",
            &conn,
        )?;
        let deleted = super::set_deleted_at(wallet.id, Some(OffsetDateTime::now_utc()), &conn)?;
        assert!(deleted.deleted_at.is_some());
        let restored = super::set_deleted_at(wallet.id, None, &conn)?;
        assert!(restored.deleted_at.is_none());
        Ok(())
    }

    #[test]
    fn set_cached_snapshot_persists_balance_and_tx() -> Result<()> {
        let conn = conn();
        let wallet = super::insert(
            "foo",
            "xpub0000000000000000000000000000000000000000000000000000000000000000",
            &conn,
        )?;
        let cached_at = OffsetDateTime::now_utc();
        let tx = vec![super::CachedTx {
            id: "tx1".into(),
            received: 100,
            sent: 30,
            delta: 70,
        }];
        let updated = super::set_cached_snapshot(wallet.id, 1234, &tx, cached_at, &conn)?;
        assert_eq!(updated.cached_balance_sats, 1234);
        assert_eq!(updated.cached_tx.len(), 1);
        assert_eq!(updated.cached_tx[0].id, "tx1");
        assert!(updated.cached_at.is_some());
        Ok(())
    }
}
