use super::schema::{self, AccessToken, AccessTokenInfo};
use crate::db::main::user::schema::Role;
use crate::Result;
use rusqlite::{params, Connection};
use schema::Columns::*;
use schema::TABLE;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub fn insert(
    user_id: i64,
    name: &str,
    secret: &str,
    roles: &[Role],
    conn: &Connection,
) -> Result<AccessToken> {
    insert_with_import_origins(user_id, name, secret, roles, &[], conn)
}

pub fn insert_with_import_origins(
    user_id: i64,
    name: &str,
    secret: &str,
    roles: &[Role],
    import_origins: &[String],
    conn: &Connection,
) -> Result<AccessToken> {
    let roles: Vec<String> = roles.iter().map(|it| it.to_string()).collect();
    let sql = format!(
        r#"
            INSERT INTO {TABLE} ({UserId}, {Name}, {Secret}, {Roles}, {ImportOrigins})
            VALUES (?1, ?2, ?3, json(?4), json(?5))
            RETURNING {projection}
        "#,
        projection = AccessToken::projection(),
    );
    conn.query_row(
        &sql,
        params![
            user_id,
            name,
            secret,
            serde_json::to_string(&roles)?,
            serde_json::to_string(import_origins)?,
        ],
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

pub fn select_by_user_id(user_id: i64, conn: &Connection) -> Result<Vec<AccessTokenInfo>> {
    let sql = format!(
        r#"
        SELECT {projection}
        FROM {TABLE}
        WHERE {UserId} = ?1 AND {DeletedAt} IS NULL
        ORDER BY {Id} ASC
    "#,
        projection = AccessTokenInfo::projection(),
    );
    conn.prepare(&sql)?
        .query_map(params![user_id], AccessTokenInfo::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    conn: &Connection,
) -> Result<AccessToken> {
    match deleted_at {
        Some(deleted_at) => {
            let sql = format!(
                r#"
                    UPDATE {TABLE}
                    SET {DeletedAt} = ?2
                    WHERE {Id} = ?1 AND {DeletedAt} IS NULL
                    RETURNING {projection}
                "#,
                projection = AccessToken::projection(),
            );
            match conn.query_row(
                &sql,
                params![id, deleted_at.format(&Rfc3339)?],
                AccessToken::mapper(),
            ) {
                Ok(token) => Ok(token),
                Err(rusqlite::Error::QueryReturnedNoRows) => select_by_id(id, conn),
                Err(err) => Err(err.into()),
            }
        }
        None => {
            let sql = format!(
                r#"
                    UPDATE {TABLE}
                    SET {DeletedAt} = NULL
                    WHERE {Id} = ?1
                "#
            );
            conn.execute(&sql, params![id])?;
            select_by_id(id, conn)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::db::main::test::conn;
    use crate::db::main::user::schema::Role;
    use crate::Result;
    use time::OffsetDateTime;

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
        assert!(selected_token.import_origins.is_empty());
        assert!(selected_token.deleted_at.is_none());
        Ok(())
    }

    #[test]
    fn insert_with_import_origins() -> Result<()> {
        let conn = conn();
        let import_origins = vec!["square".to_string(), "coinos".to_string()];
        let inserted_token = super::insert_with_import_origins(
            2,
            "name",
            "secret",
            &[Role::PlacesSource],
            &import_origins,
            &conn,
        )?;
        let selected_token = super::select_by_id(inserted_token.id, &conn)?;
        assert_eq!(import_origins, selected_token.import_origins);
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

    #[test]
    fn select_by_user_id() -> Result<()> {
        let conn = conn();
        super::insert(1, "name_1", "secret_1", &[Role::Admin], &conn)?;
        super::insert(1, "name_2", "secret_2", &[], &conn)?;
        super::insert(2, "name_3", "secret_3", &[Role::User], &conn)?;

        let tokens_for_user_1 = super::select_by_user_id(1, &conn)?;
        assert_eq!(2, tokens_for_user_1.len());
        assert_eq!(1, tokens_for_user_1[0].id);
        assert_eq!(2, tokens_for_user_1[1].id);
        assert_eq!(Some("name_1"), tokens_for_user_1[0].label.as_deref());
        assert_eq!(Some("name_2"), tokens_for_user_1[1].label.as_deref());
        assert_eq!(vec![Role::Admin], tokens_for_user_1[0].roles);

        let tokens_for_user_2 = super::select_by_user_id(2, &conn)?;
        assert_eq!(1, tokens_for_user_2.len());
        assert_eq!(3, tokens_for_user_2[0].id);

        let tokens_for_missing_user = super::select_by_user_id(999, &conn)?;
        assert!(tokens_for_missing_user.is_empty());
        Ok(())
    }

    #[test]
    fn set_deleted_at_soft_deletes_token() -> Result<()> {
        let conn = conn();
        let token = super::insert(1, "name", "secret", &[], &conn)?;
        let updated = super::set_deleted_at(token.id, Some(OffsetDateTime::now_utc()), &conn)?;
        assert_eq!(token.id, updated.id);
        assert!(updated.deleted_at.is_some());
        assert_eq!(token.secret, updated.secret);
        Ok(())
    }

    #[test]
    fn set_deleted_at_to_none_clears_deleted_at() -> Result<()> {
        let conn = conn();
        let token = super::insert(1, "name", "secret", &[], &conn)?;
        super::set_deleted_at(token.id, Some(OffsetDateTime::now_utc()), &conn)?;
        let restored = super::set_deleted_at(token.id, None, &conn)?;
        assert!(restored.deleted_at.is_none());
        Ok(())
    }
}
