use super::schema::{self, Boost, Columns};
use crate::Result;
use rusqlite::{named_params, params, Connection};

pub fn insert(
    admin_id: i64,
    element_id: i64,
    duration_days: i64,
    conn: &Connection,
) -> Result<Boost> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({admin_id}, {element_id}, {duration_days}) 
            VALUES (:admin_id, :element_id, :duration_days)
        "#,
        table = schema::TABLE_NAME,
        admin_id = Columns::AdminId.as_str(),
        element_id = Columns::ElementId.as_str(),
        duration_days = Columns::DurationDays.as_str(),
    );
    conn.execute(&sql, named_params! { ":admin_id": admin_id, ":element_id": element_id, ":duration_days": duration_days })?;
    select_by_id(conn.last_insert_rowid(), conn)
}

pub fn select_all(conn: &Connection) -> Result<Vec<Boost>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            ORDER BY {updated_at}, {id}
        "#,
        projection = Boost::projection(),
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map({}, Boost::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<Boost> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = Boost::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![id], Boost::mapper())
        .map_err(Into::into)
}
