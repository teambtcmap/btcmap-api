use super::schema::{self, Boost, Columns};
use crate::Result;
use rusqlite::{named_params, Connection};

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
            RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        admin_id = Columns::AdminId.as_str(),
        element_id = Columns::ElementId.as_str(),
        duration_days = Columns::DurationDays.as_str(),
        projection = Boost::projection(),
    );
    let params = named_params! {
        ":admin_id": admin_id,
        ":element_id": element_id,
        ":duration_days": duration_days
    };
    conn.query_row(&sql, params, Boost::mapper())
        .map_err(Into::into)
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

#[cfg(test)]
mod test {
    use crate::{db::test::conn, Result};

    #[test]
    fn insert() -> Result<()> {
        let conn = conn();
        let boost = super::insert(1, 2, 3, &conn)?;
        assert_eq!(Some(&boost), super::select_all(&conn)?.first());
        Ok(())
    }
}
