use super::schema;
use super::schema::Columns;
use crate::Result;
use rusqlite::{params, Connection, Row};

pub fn insert(
    user_id: i64,
    name: &str,
    secret: &str,
    roles: &[String],
    conn: &Connection,
) -> Result<i64> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({user_id}, {name}, {secret}, {roles})
            VALUES (?1, ?2, ?3, json(?4))
        "#,
        table = schema::NAME,
        user_id = Columns::UserId.as_str(),
        name = Columns::Name.as_str(),
        secret = Columns::Secret.as_str(),
        roles = Columns::Roles.as_str(),
    );
    conn.execute(
        &sql,
        params![user_id, name, secret, serde_json::to_string(roles)?],
    )?;
    Ok(conn.last_insert_rowid())
}

#[cfg(test)]
pub fn select_all(conn: &Connection) -> Result<Vec<AccessToken>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
        "#,
        projection = AccessToken::projection(),
        table = schema::NAME,
    );
    conn.prepare(&sql)?
        .query_map({}, AccessToken::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

#[cfg(test)]
pub fn select_by_id(id: i64, conn: &Connection) -> Result<AccessToken> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = AccessToken::projection(),
        table = schema::NAME,
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![id], AccessToken::mapper())
        .map_err(Into::into)
}

pub fn select_by_secret(secret: &str, conn: &Connection) -> Result<AccessToken> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {secret} = ?1
        "#,
        projection = AccessToken::projection(),
        table = schema::NAME,
        secret = Columns::Secret.as_str(),
    );
    conn.query_row(&sql, params![secret], AccessToken::mapper())
        .map_err(Into::into)
}

#[cfg(test)]
pub fn set_roles(token_id: i64, roles: &[String], conn: &Connection) -> Result<()> {
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
    conn.execute(&sql, params![serde_json::to_string(roles)?, token_id])?;
    Ok(())
}

#[allow(dead_code)]
pub struct AccessToken {
    pub id: i64,
    pub user_id: i64,
    pub name: Option<String>,
    pub secret: String,
    pub roles: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl AccessToken {
    fn projection() -> String {
        [
            Columns::Id,
            Columns::UserId,
            Columns::Name,
            Columns::Secret,
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
            Ok(AccessToken {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                secret: row.get(3)?,
                roles: serde_json::from_value(row.get(4)?).unwrap_or_default(),
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                deleted_at: row.get(7)?,
            })
        }
    }
}

#[cfg(test)]
mod test {
    use crate::db;
    use crate::{test::mock_conn, Result};

    #[test]
    fn insert() -> Result<()> {
        let token_name = "name";
        let token_secret = "secret";
        let conn = mock_conn();
        let user_id = db::user::queries::insert("", "", &conn)?;
        let token_id = super::insert(user_id, token_name, token_secret, &[], &conn)?;
        let token = super::select_by_id(token_id, &conn)?;
        assert_eq!(token_id, token.id);
        assert_eq!(token_name, token.name.unwrap());
        assert_eq!(token_secret, token.secret);
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = mock_conn();
        let user_id = db::user::queries::insert("", "", &conn)?;
        let token_1_id = super::insert(user_id, "name_1", "pwd_1", &[], &conn)?;
        let token_2_id = super::insert(user_id, "name_2", "pwd_2", &[], &conn)?;
        let query_res = super::select_all(&conn)?;
        assert_eq!(2, query_res.len());
        assert_eq!(token_1_id, query_res.first().unwrap().id);
        assert_eq!(token_2_id, query_res.last().unwrap().id);
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        let user_id = db::user::queries::insert("", "", &conn)?;
        let token_id = super::insert(user_id, "name", "pwd", &[], &conn)?;
        let res_token = super::select_by_id(token_id, &conn)?;
        assert_eq!(token_id, res_token.id);
        Ok(())
    }

    #[test]
    fn select_by_secret() -> Result<()> {
        let conn = mock_conn();
        let secret = "xxx";
        let user_id = db::user::queries::insert("", "", &conn)?;
        let token_id = super::insert(user_id, "", secret, &[], &conn)?;
        let query_res = super::select_by_secret(secret, &conn)?;
        assert_eq!(token_id, query_res.id);
        assert_eq!(secret, query_res.secret);
        Ok(())
    }

    #[test]
    fn set_roles() -> Result<()> {
        let conn = mock_conn();
        let user_id = db::user::queries::insert("", "", &conn)?;
        let token_id = super::insert(user_id, "name", "pwd", &[], &conn)?;
        let roles = vec!["action_1".into(), "action_2".into()];
        super::set_roles(token_id, &roles, &conn)?;
        assert_eq!(roles, super::select_by_id(token_id, &conn)?.roles,);
        Ok(())
    }
}
