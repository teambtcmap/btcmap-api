use super::schema::Columns;
use super::schema::{self, User};
use crate::Result;
use rusqlite::{params, Connection};
use tracing::warn;

pub fn insert(name: &str, password: &str, conn: &Connection) -> Result<i64> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({name}, {password})
            VALUES (?1, ?2)
        "#,
        table = schema::TABLE_NAME,
        name = Columns::Name.as_str(),
        password = Columns::Password.as_str(),
    );
    conn.execute(&sql, params![name, password])?;
    Ok(conn.last_insert_rowid())
}

#[cfg(test)]
pub fn select_all(conn: &Connection) -> Result<Vec<User>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
        "#,
        projection = User::projection(),
        table = schema::TABLE_NAME,
    );
    conn.prepare(&sql)?
        .query_map({}, User::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<User> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = User::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![id], User::mapper())
        .map_err(Into::into)
}

pub fn select_by_name(name: &str, conn: &Connection) -> Result<User> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {name} = ?1
        "#,
        projection = User::projection(),
        table = schema::TABLE_NAME,
        name = Columns::Name.as_str(),
    );
    conn.query_row(&sql, params![name], User::mapper())
        .map_err(Into::into)
}

pub fn set_password(id: i64, password: impl Into<String>, conn: &Connection) -> Result<()> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {password} = ?1
            WHERE {id} = ?2
        "#,
        table = schema::TABLE_NAME,
        password = Columns::Password.as_str(),
        id = Columns::Id.as_str(),
    );
    warn!(sql);
    conn.execute(&sql, params![password.into(), id])?;
    Ok(())
}

pub fn set_roles(admin_id: i64, roles: &[String], conn: &Connection) -> Result<()> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {roles} = json(?1)
            WHERE {id} = ?2
        "#,
        table = schema::TABLE_NAME,
        roles = Columns::Roles.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![serde_json::to_string(roles)?, admin_id])?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::{test::mock_conn, Result};

    #[test]
    fn insert() -> Result<()> {
        let admin_name = "name";
        let admin_pwd = "pwd";
        let conn = mock_conn();
        let admin_id = super::insert(admin_name, admin_pwd, &conn)?;
        let res_admin = super::select_by_id(admin_id, &conn)?;
        assert_eq!(admin_id, res_admin.id);
        assert_eq!(admin_name, res_admin.name);
        assert_eq!(admin_pwd, res_admin.password);
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = mock_conn();
        let admin_1_id = super::insert("name_1", "pwd_1", &conn)?;
        let admin_2_id = super::insert("name_2", "pwd_2", &conn)?;
        let query_res = super::select_all(&conn)?;
        assert_eq!(2, query_res.len());
        assert_eq!(admin_1_id, query_res.first().unwrap().id);
        assert_eq!(admin_2_id, query_res.last().unwrap().id);
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        let admin_id = super::insert("name", "pwd", &conn)?;
        let res_admin = super::select_by_id(admin_id, &conn)?;
        assert_eq!(admin_id, res_admin.id);
        Ok(())
    }

    #[test]
    fn select_by_name() -> Result<()> {
        let admin_name = "admin_1";
        let conn = mock_conn();
        let admin_id = super::insert(admin_name, "", &conn)?;
        let res_admin = super::select_by_name(admin_name, &conn)?;
        assert_eq!(admin_id, res_admin.id);
        assert_eq!(admin_name, res_admin.name);
        Ok(())
    }

    #[test]
    fn set_roles() -> Result<()> {
        let conn = mock_conn();
        let admin_id = super::insert("name", "pwd", &conn)?;
        let roles = vec!["action_1".into(), "action_2".into()];
        super::set_roles(admin_id, &roles, &conn)?;
        assert_eq!(roles, super::select_by_id(admin_id, &conn)?.roles,);
        Ok(())
    }
}
