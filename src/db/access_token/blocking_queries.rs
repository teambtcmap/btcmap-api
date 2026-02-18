use super::schema::Columns;
use super::schema::{self, AccessToken};
use crate::db::user::schema::Role;
use crate::Result;
use rusqlite::{params, Connection};

pub fn insert(
    user_id: i64,
    name: &str,
    secret: &str,
    roles: &[Role],
    conn: &Connection,
) -> Result<AccessToken> {
    let roles: Vec<String> = roles.iter().map(|it| it.to_string()).collect();
    let sql = format!(
        r#"
            INSERT INTO {table} ({user_id}, {name}, {secret}, {roles})
            VALUES (?1, ?2, ?3, json(?4))
            RETURNING {projection}
        "#,
        table = schema::NAME,
        user_id = Columns::UserId.as_str(),
        name = Columns::Name.as_str(),
        secret = Columns::Secret.as_str(),
        roles = Columns::Roles.as_str(),
        projection = AccessToken::projection(),
    );
    conn.query_row(
        &sql,
        params![user_id, name, secret, serde_json::to_string(&roles)?],
        AccessToken::mapper(),
    )
    .map_err(Into::into)
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
pub fn set_roles(token_id: i64, roles: &[Role], conn: &Connection) -> Result<()> {
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
    let roles: Vec<String> = roles.iter().map(|role| role.to_string()).collect();
    conn.execute(&sql, params![serde_json::to_string(&roles)?, token_id])?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::db::test::conn;
    use crate::db::user::schema::Role;
    use crate::Result;

    #[test]
    fn insert() -> Result<()> {
        let conn = conn();
        let user = crate::db::user::blocking_queries::insert("test_user", "password", &conn)?;
        let name = "name";
        let secret = "secret";
        let roles = vec![Role::Admin];

        let inserted_token = super::insert(user.id, name, secret, &roles, &conn)?;
        let selected_token = super::select_by_id(inserted_token.id, &conn)?;

        assert_eq!(selected_token, inserted_token);

        assert_eq!(1, selected_token.id);
        assert_eq!(user.id, selected_token.user_id);
        assert_eq!(Some(name), selected_token.name.as_deref());
        assert_eq!(secret, selected_token.secret);
        assert_eq!(roles, selected_token.roles);
        assert!(selected_token.deleted_at.is_none());

        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = conn();
        let user = crate::db::user::blocking_queries::insert("test_user", "password", &conn)?;
        let token_1 = super::insert(user.id, "name_1", "pwd_1", &[], &conn)?;
        let token_2 = super::insert(user.id, "name_2", "pwd_2", &[], &conn)?;
        let query_res = super::select_all(&conn)?;
        assert_eq!(2, query_res.len());
        assert_eq!(&token_1, query_res.first().unwrap());
        assert_eq!(&token_2, query_res.last().unwrap());
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = conn();
        let user = crate::db::user::blocking_queries::insert("test_user", "password", &conn)?;
        let insert_res = super::insert(user.id, "name", "pwd", &[], &conn)?;
        let select_res = super::select_by_id(insert_res.id, &conn)?;
        assert_eq!(insert_res, select_res);
        Ok(())
    }

    #[test]
    fn select_by_secret() -> Result<()> {
        let conn = conn();
        let user = crate::db::user::blocking_queries::insert("test_user", "password", &conn)?;
        let secret = "xxx";
        let token = super::insert(user.id, "", secret, &[], &conn)?;
        let select_res = super::select_by_secret(secret, &conn)?;
        assert_eq!(token, select_res);
        Ok(())
    }

    #[test]
    fn set_roles() -> Result<()> {
        let conn = conn();
        let user = crate::db::user::blocking_queries::insert("test_user", "password", &conn)?;
        let token = super::insert(user.id, "name", "pwd", &[], &conn)?;
        let roles = vec![Role::User, Role::Admin];
        super::set_roles(token.id, &roles, &conn)?;
        assert_eq!(roles, super::select_by_id(token.id, &conn)?.roles);
        Ok(())
    }
}
