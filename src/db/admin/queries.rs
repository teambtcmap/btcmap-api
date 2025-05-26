use super::schema;
use super::schema::Columns;
use crate::Result;
use rusqlite::{params, Connection, Row};
use tracing::warn;

pub fn insert(name: &str, password: &str, conn: &Connection) -> Result<i64> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({name}, {password})
            VALUES (?1, ?2)
        "#,
        table = schema::NAME,
        name = Columns::Name.as_str(),
        password = Columns::Password.as_str(),
    );
    conn.execute(&sql, params![name, password])?;
    Ok(conn.last_insert_rowid())
}

pub fn select_all(conn: &Connection) -> Result<Vec<Admin>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
        "#,
        projection = Admin::projection(),
        table = schema::NAME,
    );
    conn.prepare(&sql)?
        .query_map({}, Admin::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<Admin> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = Admin::projection(),
        table = schema::NAME,
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![id], Admin::mapper())
        .map_err(Into::into)
}

pub fn select_by_name(name: &str, conn: &Connection) -> Result<Admin> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {name} = ?1
        "#,
        projection = Admin::projection(),
        table = schema::NAME,
        name = Columns::Name.as_str(),
    );
    conn.query_row(&sql, params![name], Admin::mapper())
        .map_err(Into::into)
}

pub fn set_password(id: i64, password: impl Into<String>, conn: &Connection) -> Result<()> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {password} = ?1
            WHERE {id} = ?2
        "#,
        table = schema::NAME,
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
        table = schema::NAME,
        roles = Columns::Roles.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![serde_json::to_string(roles)?, admin_id])?;
    Ok(())
}

pub struct Admin {
    pub id: i64,
    pub name: String,
    #[allow(dead_code)]
    pub password: String,
    pub roles: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl Admin {
    fn projection() -> String {
        [
            Columns::Id,
            Columns::Name,
            Columns::Password,
            Columns::Roles,
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
            Ok(Admin {
                id: row.get(0)?,
                name: row.get(1)?,
                password: row.get(2)?,
                roles: serde_json::from_value(row.get(3)?).unwrap_or_default(),
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                deleted_at: row.get(6)?,
            })
        }
    }
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
