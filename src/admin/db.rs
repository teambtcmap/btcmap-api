use crate::Result;
use deadpool_sqlite::Pool;
use rusqlite::{params, Connection, Row};

const TABLE_NAME: &str = "admin";

enum Columns {
    Id,
    Name,
    Password,
    AllowedActions,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

impl Columns {
    fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::Name => "name",
            Columns::Password => "password",
            Columns::AllowedActions => "allowed_actions",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::DeletedAt => "deleted_at",
        }
    }

    fn projection_full() -> String {
        vec![
            Self::Id,
            Self::Name,
            Self::Password,
            Self::AllowedActions,
            Self::CreatedAt,
            Self::UpdatedAt,
            Self::DeletedAt,
        ]
        .iter()
        .map(Self::as_str)
        .collect::<Vec<_>>()
        .join(", ")
    }

    fn mapper_full() -> fn(&Row) -> rusqlite::Result<Admin> {
        |row: &Row| -> rusqlite::Result<Admin> {
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
    pub async fn insert(
        name: impl Into<String>,
        password: impl Into<String>,
        pool: &Pool,
    ) -> Result<Self> {
        let name = name.into();
        let password = password.into();
        pool.get()
            .await?
            .interact(move |conn| Self::_insert(&name, &password, conn))
            .await?
    }

    fn _insert(name: &str, password: &str, conn: &Connection) -> Result<Self> {
        let sql = format!(
            r#"
                INSERT INTO {table} ({name}, {password})
                VALUES (?1, ?2)
            "#,
            table = TABLE_NAME,
            name = Columns::Name.as_str(),
            password = Columns::Password.as_str(),
        );
        conn.execute(&sql, params![name, password])?;
        Self::_select_by_id(conn.last_insert_rowid(), conn)
    }

    pub async fn select_by_id(id: i64, pool: &Pool) -> Result<Self> {
        pool.get()
            .await?
            .interact(move |conn| Self::_select_by_id(id, conn))
            .await?
    }

    fn _select_by_id(id: i64, conn: &Connection) -> Result<Self> {
        let sql = format!(
            r#"
                SELECT {projection}
                FROM {table}
                WHERE {id} = ?1
            "#,
            projection = Columns::projection_full(),
            table = TABLE_NAME,
            id = Columns::Id.as_str(),
        );
        conn.query_row(&sql, params![id], Columns::mapper_full())
            .map_err(Into::into)
    }

    pub async fn select_by_name(name: impl Into<String>, pool: &Pool) -> Result<Self> {
        let name = name.into();
        pool.get()
            .await?
            .interact(move |conn| Self::_select_by_name(&name, conn))
            .await?
    }

    fn _select_by_name(name: &str, conn: &Connection) -> Result<Self> {
        let sql = format!(
            r#"
                SELECT {projection}
                FROM {table}
                WHERE {name} = ?1
            "#,
            projection = Columns::projection_full(),
            table = TABLE_NAME,
            name = Columns::Name.as_str(),
        );
        conn.query_row(&sql, params![name], Columns::mapper_full())
            .map_err(Into::into)
    }

    pub async fn select_by_password(password: impl Into<String>, pool: &Pool) -> Result<Self> {
        let password = password.into();
        pool.get()
            .await?
            .interact(move |conn| Self::_select_by_password(&password, conn))
            .await?
    }

    fn _select_by_password(password: &str, conn: &Connection) -> Result<Self> {
        let sql = format!(
            r#"
                SELECT {projection}
                FROM {table}
                WHERE {password} = ?1
            "#,
            projection = Columns::projection_full(),
            table = TABLE_NAME,
            password = Columns::Password.as_str(),
        );
        conn.query_row(&sql, params![password], Columns::mapper_full())
            .map_err(Into::into)
    }

    pub async fn update_allowed_actions(
        admin_id: i64,
        allowed_actions: &[String],
        pool: &Pool,
    ) -> Result<Self> {
        let allowed_actions = allowed_actions.to_vec();
        pool.get()
            .await?
            .interact(move |conn| Self::_update_allowed_actions(admin_id, &allowed_actions, conn))
            .await?
    }

    pub fn _update_allowed_actions(
        id: i64,
        allowed_actions: &[String],
        conn: &Connection,
    ) -> Result<Self> {
        let sql = format!(
            r#"
                UPDATE {table}
                SET {allowed_actions} = json(?1)
                WHERE {id} = ?2
            "#,
            table = TABLE_NAME,
            allowed_actions = Columns::AllowedActions.as_str(),
            id = Columns::Id.as_str(),
        );
        conn.execute(&sql, params![serde_json::to_string(allowed_actions)?, id])?;
        Self::_select_by_id(id, conn)
    }
}

#[cfg(test)]
mod test {
    use super::Admin;
    use crate::{test::mock_pool, Result};
    use actix_web::test;

    #[test]
    async fn insert() -> Result<()> {
        let pool = mock_pool().await;
        let admin = Admin::insert("name", "pwd", &pool).await?;
        let res_admin = Admin::select_by_id(admin.id, &pool).await?;
        assert_eq!(admin.id, res_admin.id);
        assert_eq!(admin.name, res_admin.name);
        assert_eq!(admin.password, res_admin.password);
        Ok(())
    }

    #[test]
    async fn select_by_id() -> Result<()> {
        let pool = mock_pool().await;
        let admin = Admin::insert("name", "pwd", &pool).await?;
        let res_admin = Admin::select_by_id(admin.id, &pool).await?;
        assert_eq!(admin.id, res_admin.id);
        Ok(())
    }

    #[test]
    async fn select_by_name() -> Result<()> {
        let pool = mock_pool().await;
        let admin = Admin::insert("name", "pwd", &pool).await?;
        let res_admin = Admin::select_by_id(admin.id, &pool).await?;
        assert_eq!(admin.id, res_admin.id);
        assert_eq!(admin.name, res_admin.name);
        Ok(())
    }

    #[test]
    async fn select_by_password() -> Result<()> {
        let pool = mock_pool().await;
        let password = "pwd";
        let admin = Admin::insert("name", password, &pool).await?;
        let res_admin = Admin::select_by_password(password, &pool).await?;
        assert_eq!(admin.id, res_admin.id);
        assert_eq!(admin.password, res_admin.password);
        Ok(())
    }

    #[test]
    async fn update_allowed_actions() -> Result<()> {
        let pool = mock_pool().await;
        let admin = Admin::insert("name", "pwd", &pool).await?;
        let actions = vec!["action_1".into(), "action_2".into()];
        Admin::update_allowed_actions(admin.id, &actions, &pool).await?;
        assert_eq!(
            actions,
            Admin::select_by_id(admin.id, &pool).await?.allowed_actions,
        );
        Ok(())
    }
}
