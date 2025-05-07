use super::schema;
use super::schema::Columns;
use crate::Result;
use rusqlite::{params, Connection, Row};

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

pub fn select_by_password(password: &str, conn: &Connection) -> Result<Admin> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {password} = ?1
        "#,
        projection = Admin::projection(),
        table = schema::NAME,
        password = Columns::Password.as_str(),
    );
    conn.query_row(&sql, params![password], Admin::mapper())
        .map_err(Into::into)
}

pub fn update_allowed_actions(
    id: i64,
    new_allowed_actions: &[String],
    conn: &Connection,
) -> Result<()> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {allowed_actions} = json(?1)
            WHERE {id} = ?2
        "#,
        table = schema::NAME,
        allowed_actions = Columns::AllowedActions.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(
        &sql,
        params![serde_json::to_string(new_allowed_actions)?, id],
    )?;
    Ok(())
}

pub struct Admin {
    pub id: i64,
    pub name: String,
    #[allow(dead_code)]
    pub password: String,
    pub allowed_actions: Vec<String>,
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
            Columns::AllowedActions,
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
                allowed_actions: serde_json::from_value(row.get(3)?).unwrap_or_default(),
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
    fn select_by_password() -> Result<()> {
        let conn = mock_conn();
        let password = "pwd";
        let admin_id = super::insert("", password, &conn)?;
        let query_res = super::select_by_password(password, &conn)?;
        assert_eq!(admin_id, query_res.id);
        assert_eq!(password, query_res.password);
        Ok(())
    }

    #[test]
    fn update_allowed_actions() -> Result<()> {
        let conn = mock_conn();
        let admin_id = super::insert("name", "pwd", &conn)?;
        let actions = vec!["action_1".into(), "action_2".into()];
        super::update_allowed_actions(admin_id, &actions, &conn)?;
        assert_eq!(
            actions,
            super::select_by_id(admin_id, &conn)?.allowed_actions,
        );
        Ok(())
    }
}
