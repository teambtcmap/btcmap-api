use super::schema::Role;
use super::schema::{self, User};
use crate::Result;
use rusqlite::{params, Connection};
use schema::Columns::*;
use schema::TABLE_NAME as TABLE;

pub fn insert(name: &str, password: &str, conn: &Connection) -> Result<User> {
    conn.query_row(
        &format!(
            r#"
                INSERT INTO {TABLE} ({Name}, {Password})
                VALUES (?1, ?2)
                RETURNING {projection}
            "#,
            projection = User::projection(),
        ),
        params![name, password],
        User::mapper(),
    )
    .map_err(Into::into)
}

#[allow(dead_code)]
pub fn select_all(conn: &Connection) -> Result<Vec<User>> {
    conn.prepare(&format!(
        r#"
            SELECT {projection}
            FROM {TABLE}
        "#,
        projection = User::projection(),
    ))?
    .query_map({}, User::mapper())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<User> {
    conn.query_row(
        &format!(
            r#"
                SELECT {projection}
                FROM {TABLE}
                WHERE {Id} = ?1
            "#,
            projection = User::projection(),
        ),
        params![id],
        User::mapper(),
    )
    .map_err(Into::into)
}

pub fn select_by_name(name: &str, conn: &Connection) -> Result<User> {
    conn.query_row(
        &format!(
            r#"
                SELECT {projection}
                FROM {TABLE}
                WHERE {Name} = ?1
            "#,
            projection = User::projection(),
        ),
        params![name],
        User::mapper(),
    )
    .map_err(Into::into)
}

pub fn set_password(id: i64, password: impl Into<String>, conn: &Connection) -> Result<usize> {
    conn.execute(
        &format!(
            r#"
                UPDATE {TABLE}
                SET {Password} = ?1
                WHERE {Id} = ?2
            "#,
        ),
        params![password.into(), id],
    )
    .map_err(Into::into)
}

pub fn set_roles(admin_id: i64, roles: &[Role], conn: &Connection) -> Result<User> {
    let roles: Vec<String> = roles.iter().map(|role| role.to_string()).collect();
    conn.query_row(
        &format!(
            r#"
                UPDATE {TABLE}
                SET {Roles} = json(?1)
                WHERE {Id} = ?2
                RETURNING {projection}
            "#,
            projection = User::projection(),
        ),
        params![serde_json::to_string(&roles)?, admin_id],
        User::mapper(),
    )
    .map_err(Into::into)
}

#[cfg(test)]
mod test {
    use super::schema::Role;
    use crate::{db::main::test::conn, Result};

    #[test]
    fn insert() -> Result<()> {
        let admin_name = "name";
        let admin_pwd = "pwd";
        let conn = conn();
        let admin_id = super::insert(admin_name, admin_pwd, &conn)?.id;
        let res_admin = super::select_by_id(admin_id, &conn)?;
        assert_eq!(admin_id, res_admin.id);
        assert_eq!(admin_name, res_admin.name);
        assert_eq!(admin_pwd, res_admin.password);
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = conn();
        let admin_1_id = super::insert("name_1", "pwd_1", &conn)?.id;
        let admin_2_id = super::insert("name_2", "pwd_2", &conn)?.id;
        let query_res = super::select_all(&conn)?;
        assert_eq!(2, query_res.len());
        assert_eq!(admin_1_id, query_res.first().unwrap().id);
        assert_eq!(admin_2_id, query_res.last().unwrap().id);
        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = conn();
        let admin_id = super::insert("name", "pwd", &conn)?.id;
        let res_admin = super::select_by_id(admin_id, &conn)?;
        assert_eq!(admin_id, res_admin.id);
        Ok(())
    }

    #[test]
    fn select_by_name() -> Result<()> {
        let admin_name = "admin_1";
        let conn = conn();
        let admin_id = super::insert(admin_name, "", &conn)?.id;
        let res_admin = super::select_by_name(admin_name, &conn)?;
        assert_eq!(admin_id, res_admin.id);
        assert_eq!(admin_name, res_admin.name);
        Ok(())
    }

    #[test]
    fn set_roles() -> Result<()> {
        let conn = conn();
        let admin_id = super::insert("name", "pwd", &conn)?.id;
        let roles = vec![Role::User, Role::Admin];
        super::set_roles(admin_id, &roles, &conn)?;
        assert_eq!(roles, super::select_by_id(admin_id, &conn)?.roles,);
        Ok(())
    }
}
