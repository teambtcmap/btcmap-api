use crate::{error, Result};
use deadpool_sqlite::Pool;
use rusqlite::{named_params, Connection, OptionalExtension, Row};

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
const MAPPER_PROJECTION: &str = "id, name, allowed_actions";

impl Admin {
    pub async fn insert_async(
        name: impl Into<String>,
        password: impl Into<String>,
        pool: &Pool,
    ) -> Result<Admin> {
        let name = name.into();
        let password = password.into();
        pool.get()
            .await?
            .interact(move |conn| Admin::insert(name, password, conn))
            .await?
    }

    pub fn insert(
        name: impl Into<String>,
        password: impl Into<String>,
        conn: &Connection,
    ) -> Result<Admin> {
        let password = password.into();
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
        conn.execute(
            &sql,
            named_params! {
                ":name": name.into(),
                ":password": &password,
            },
        )?;
        Admin::select_by_password(&password, conn)?
            .ok_or(error::select_after_insert_failed("admin"))
    }

    pub fn select_by_id(id: i64, conn: &Connection) -> Result<Option<Admin>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_ID} = :{COL_ID};
            "#
        );
        conn.query_row(&sql, named_params! { ":id": id }, mapper())
            .optional()
            .map_err(Into::into)
    }

    pub async fn select_by_name_async(
        name: impl Into<String>,
        pool: &Pool,
    ) -> Result<Option<Admin>> {
        let name = name.into();
        pool.get()
            .await?
            .interact(move |conn| Admin::select_by_name(name, conn))
            .await?
    }

    pub fn select_by_name(name: impl Into<String>, conn: &Connection) -> Result<Option<Admin>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_NAME} = :{COL_NAME};
            "#
        );
        conn.query_row(&sql, named_params! { ":name": name.into() }, mapper())
            .optional()
            .map_err(Into::into)
    }

    pub async fn select_by_password_async(
        password: impl Into<String>,
        pool: &Pool,
    ) -> Result<Option<Admin>> {
        let password = password.into();
        pool.get()
            .await?
            .interact(move |conn| Admin::select_by_password(password, conn))
            .await?
    }

    pub fn select_by_password(
        password: impl Into<String>,
        conn: &Connection,
    ) -> Result<Option<Admin>> {
        let sql = format!(
            r#"
                SELECT {MAPPER_PROJECTION}
                FROM {TABLE_NAME}
                WHERE {COL_PASSWORD} = :{COL_PASSWORD}
            "#
        );
        conn.query_row(
            &sql,
            named_params! { ":password": password.into() },
            mapper(),
        )
        .optional()
        .map_err(Into::into)
    }

    pub async fn update_allowed_actions_async(
        id: i64,
        allowed_actions: &[String],
        pool: &Pool,
    ) -> Result<Option<Admin>> {
        let allowed_actions = allowed_actions.to_vec();
        pool.get()
            .await?
            .interact(move |conn| Admin::update_allowed_actions(id, &allowed_actions, conn))
            .await?
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
        conn.execute(
            &sql,
            named_params! {
                ":id": id,
                ":allowed_actions": serde_json::to_string(allowed_actions)?,
            },
        )?;
        Admin::select_by_id(id, conn)
    }
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

#[cfg(test)]
mod test {
    use super::Admin;
    use crate::{test::mock_conn, Result};

    #[test]
    fn insert() -> Result<()> {
        let conn = mock_conn();
        let admin = Admin::insert("name", "pwd", &conn)?;
        assert_eq!(Some(admin), Admin::select_by_id(1, &conn)?);
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        let admin = Admin::insert("name", "pwd", &conn)?;
        assert_eq!(Some(admin), Admin::select_by_id(1, &conn)?);
        Ok(())
    }

    #[test]
    fn select_by_name() -> Result<()> {
        let conn = mock_conn();
        let name = "name";
        let admin = Admin::insert(name, "pwd", &conn)?;
        assert_eq!(Some(admin), Admin::select_by_name(name, &conn)?);
        Ok(())
    }

    #[test]
    fn select_by_password() -> Result<()> {
        let conn = mock_conn();
        let password = "pwd";
        let admin = Admin::insert("name", password, &conn)?;
        assert_eq!(Some(admin), Admin::select_by_password(password, &conn)?);
        Ok(())
    }

    #[test]
    fn update_allowed_actions() -> Result<()> {
        let conn = mock_conn();
        let admin = Admin::insert("name", "pwd", &conn)?;
        let actions = vec!["action_1".into(), "action_2".into()];
        Admin::update_allowed_actions(admin.id, &actions, &conn)?;
        assert_eq!(
            Some(actions),
            Admin::select_by_id(admin.id, &conn)?.map(|it| it.allowed_actions),
        );
        Ok(())
    }
}
