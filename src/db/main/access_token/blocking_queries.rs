use super::schema::{self, AccessToken};
use crate::db::main::user::schema::Role;
use crate::Result;
use rusqlite::{params, Connection};
use schema::Columns::*;
use schema::TABLE;

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
            INSERT INTO {TABLE} ({UserId}, {Name}, {Secret}, {Roles})
            VALUES (?1, ?2, ?3, json(?4))
            RETURNING {projection}
        "#,
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
            FROM {TABLE}
        "#,
        projection = AccessToken::projection(),
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
            FROM {TABLE}
            WHERE {Id} = ?1
        "#,
        projection = AccessToken::projection(),
    );
    conn.query_row(&sql, params![id], AccessToken::mapper())
        .map_err(Into::into)
}

pub fn select_by_secret(secret: &str, conn: &Connection) -> Result<AccessToken> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {TABLE}
            WHERE {Secret} = ?1 AND {DeletedAt} IS NULL
        "#,
        projection = AccessToken::projection(),
    );
    conn.query_row(&sql, params![secret], AccessToken::mapper())
        .map_err(Into::into)
}

#[cfg(test)]
mod test {
    use crate::db::main::test::conn;
    use crate::db::main::user::schema::Role;
    use crate::Result;

    #[test]
    fn insert() -> Result<()> {
        let conn = conn();
        let name = "name";
        let secret = "secret";
        let roles = vec![Role::Admin];
        let inserted_token = super::insert(2, name, secret, &roles, &conn)?;
        let selected_token = super::select_by_id(inserted_token.id, &conn)?;
        assert_eq!(inserted_token, selected_token);
        assert_eq!(selected_token, inserted_token);
        assert_eq!(1, selected_token.id);
        assert_eq!(2, selected_token.user_id);
        assert_eq!(Some(name), selected_token.name.as_deref());
        assert_eq!(secret, selected_token.secret);
        assert_eq!(roles, selected_token.roles);
        assert!(selected_token.deleted_at.is_none());
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = conn();
        let token_1 = super::insert(1, "name_1", "pwd_1", &[], &conn)?;
        let token_2 = super::insert(1, "name_2", "pwd_2", &[], &conn)?;
        let query_res = super::select_all(&conn)?;
        assert_eq!(2, query_res.len());
        assert_eq!(&token_1, query_res.first().unwrap());
        assert_eq!(&token_2, query_res.last().unwrap());
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = conn();
        let insert_res = super::insert(1, "name", "pwd", &[], &conn)?;
        let select_res = super::select_by_id(insert_res.id, &conn)?;
        assert_eq!(insert_res, select_res);
        Ok(())
    }

    #[test]
    fn select_by_secret() -> Result<()> {
        let conn = conn();
        let secret = "xxx";
        let token = super::insert(1, "", secret, &[], &conn)?;
        let select_res = super::select_by_secret(secret, &conn)?;
        assert_eq!(token, select_res);
        Ok(())
    }
}
