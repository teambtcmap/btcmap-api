use crate::Result;
use deadpool_sqlite::Pool;
use rusqlite::{named_params, Connection, OptionalExtension, Row};
#[cfg(not(test))]
use std::thread::sleep;
#[cfg(not(test))]
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub struct Admin {
    pub id: i64,
    pub name: String,
    pub allowed_actions: Vec<String>,
}

const TABLE_NAME: &str = "admin";
const COL_ID: &str = "id";
const COL_NAME: &str = "name";
const COL_PASSWORD: &str = "password";
const COL_ALLOWED_ACTIONS: &str = "allowed_actions";
const _COL_CREATED_AT: &str = "created_at";
const _COL_UPDATED_AT: &str = "updated_at";
const _COL_DELETED_AT: &str = "deleted_at";
const MAPPER_PROJECTION: &str = "id, name, allowed_actions";

impl Admin {
    pub fn insert(name: &str, password: &str, conn: &Connection) -> Result<Option<Admin>> {
        let sql = format!(
            r#"
                INSERT INTO {TABLE_NAME} (
                    {COL_NAME},
                    {COL_PASSWORD}
                ) VALUES (
                    :{COL_NAME},
                    :{COL_PASSWORD}
                );
            "#
        );
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            &sql,
            named_params! {
                ":name": name,
                ":password": password,
            },
        )?;
        Admin::select_by_password(password, conn)
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<Admin>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_ID} = :{COL_ID};
            "#
        );
        conn.query_row(&sql, named_params! { ":id": id }, Self::mapper())
            .optional()
            .map_err(Into::into)
    }

    pub fn select_by_name(name: &str, conn: &Connection) -> Result<Option<Admin>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_NAME} = :{COL_NAME};
            "#
        );
        conn.query_row(&sql, named_params! { ":name": name }, Self::mapper())
            .optional()
            .map_err(Into::into)
    }

    pub async fn select_by_password_async(password: &str, pool: &Pool) -> Result<Option<Admin>> {
        let password = password.to_string();
        pool.get()
            .await?
            .interact(move |conn| Admin::select_by_password(&password, conn))
            .await?
    }

    pub fn select_by_password(password: &str, conn: &Connection) -> Result<Option<Admin>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_PASSWORD} = :{COL_PASSWORD}
            "#
        );
        conn.query_row(
            &sql,
            named_params! { ":password": password },
            Self::mapper(),
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn update_allowed_actions(
        id: i64,
        allowed_actions: &[String],
        conn: &Connection,
    ) -> Result<Option<Admin>> {
        let sql = format!(
            r#"
                UPDATE {TABLE_NAME}
                SET {COL_ALLOWED_ACTIONS} = json(:{COL_ALLOWED_ACTIONS})
                WHERE {COL_ID} = :{COL_ID}
            "#
        );
        #[cfg(not(test))]
        sleep(Duration::from_millis(10));
        conn.execute(
            &sql,
            named_params! {
                ":id": id,
                ":allowed_actions": serde_json::to_string(allowed_actions)?,
            },
        )?;
        Admin::select_by_id(id, conn)
    }

    const fn mapper() -> fn(&Row) -> rusqlite::Result<Admin> {
        |row: &Row| -> rusqlite::Result<Admin> {
            Ok(Admin {
                id: row.get(0)?,
                name: row.get(1)?,
                allowed_actions: serde_json::from_value(row.get(2)?).unwrap_or_default(),
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::Admin;
    use crate::{test::mock_conn, Result};

    #[test]
    fn insert() -> Result<()> {
        let conn = mock_conn();
        let admin = Admin::insert("name", "pwd", &conn)?.ok_or("can't insert admin")?;
        assert_eq!(Some(admin), Admin::select_by_id(1, &conn)?);
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        let admin = Admin::insert("name", "pwd", &conn)?.ok_or("can't insert admin")?;
        assert_eq!(Some(admin), Admin::select_by_id(1, &conn)?);
        Ok(())
    }

    #[test]
    fn select_by_name() -> Result<()> {
        let conn = mock_conn();
        let name = "name";
        let admin = Admin::insert(name, "pwd", &conn)?.ok_or("can't insert admin")?;
        assert_eq!(Some(admin), Admin::select_by_name(name, &conn)?);
        Ok(())
    }

    #[test]
    fn select_by_password() -> Result<()> {
        let conn = mock_conn();
        let password = "pwd";
        let admin = Admin::insert("name", password, &conn)?.ok_or("can't insert admin")?;
        assert_eq!(Some(admin), Admin::select_by_password(password, &conn)?);
        Ok(())
    }

    #[test]
    fn update_allowed_actions() -> Result<()> {
        let conn = mock_conn();
        let admin = Admin::insert("name", "pwd", &conn)?.ok_or("can't insert admin")?;
        let actions = vec!["action_1".into(), "action_2".into()];
        Admin::update_allowed_actions(admin.id, &actions, &conn)?;
        assert_eq!(
            Some(actions),
            Admin::select_by_id(admin.id, &conn)?.map(|it| it.allowed_actions),
        );
        Ok(())
    }
}
