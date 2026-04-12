use super::schema::Role;
use super::schema::{self, User};
use crate::Result;
use rusqlite::{params, Connection, OptionalExtension};
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

#[allow(dead_code)]
pub fn select_by_npub(npub: &str, conn: &Connection) -> Result<Option<User>> {
    conn.query_row(
        &format!(
            r#"
                SELECT {projection}
                FROM {TABLE}
                WHERE {Npub} = ?1
            "#,
            projection = User::projection(),
        ),
        params![npub],
        User::mapper(),
    )
    .optional()
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

pub fn set_name(id: i64, name: &str, conn: &Connection) -> Result<User> {
    conn.query_row(
        &format!(
            r#"
                UPDATE {TABLE}
                SET {Name} = ?1
                WHERE {Id} = ?2
                RETURNING {projection}
            "#,
            projection = User::projection(),
        ),
        params![name, id],
        User::mapper(),
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

#[allow(dead_code)]
pub fn set_saved_places(id: i64, saved_places: &[i64], conn: &Connection) -> Result<User> {
    let saved_places: String = saved_places
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    conn.query_row(
        &format!(
            r#"
                UPDATE {TABLE}
                SET {SavedPlaces} = ?1
                WHERE {Id} = ?2
                RETURNING {projection}
            "#,
            projection = User::projection(),
        ),
        params![saved_places, id],
        User::mapper(),
    )
    .map_err(Into::into)
}

#[allow(dead_code)]
pub fn set_saved_areas(id: i64, saved_areas: &[i64], conn: &Connection) -> Result<User> {
    let saved_areas: String = saved_areas
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    conn.query_row(
        &format!(
            r#"
                UPDATE {TABLE}
                SET {SavedAreas} = ?1
                WHERE {Id} = ?2
                RETURNING {projection}
            "#,
            projection = User::projection(),
        ),
        params![saved_areas, id],
        User::mapper(),
    )
    .map_err(Into::into)
}

#[allow(dead_code)]
pub fn set_npub(id: i64, npub: Option<String>, conn: &Connection) -> Result<User> {
    conn.query_row(
        &format!(
            r#"
                UPDATE {TABLE}
                SET {Npub} = ?1
                WHERE {Id} = ?2
                RETURNING {projection}
            "#,
            projection = User::projection(),
        ),
        params![npub, id],
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
    fn select_by_npub() -> Result<()> {
        let conn = conn();
        let npub_value = "npub1test123";
        let admin_id = super::insert("name", "pwd", &conn)?.id;
        super::set_npub(admin_id, Some(npub_value.to_string()), &conn)?;
        let res_admin = super::select_by_npub(npub_value, &conn)?;
        assert_eq!(admin_id, res_admin.unwrap().id);
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

    #[test]
    fn set_saved_places() -> Result<()> {
        let conn = conn();
        let admin_id = super::insert("name", "pwd", &conn)?.id;
        let saved_places = vec![1, 2, 3];
        super::set_saved_places(admin_id, &saved_places, &conn)?;
        assert_eq!(
            saved_places,
            super::select_by_id(admin_id, &conn)?.saved_places
        );
        Ok(())
    }

    #[test]
    fn set_saved_areas() -> Result<()> {
        let conn = conn();
        let admin_id = super::insert("name", "pwd", &conn)?.id;
        let saved_areas = vec![10, 20, 30];
        super::set_saved_areas(admin_id, &saved_areas, &conn)?;
        assert_eq!(
            saved_areas,
            super::select_by_id(admin_id, &conn)?.saved_areas
        );
        Ok(())
    }

    #[test]
    fn set_npub() -> Result<()> {
        let conn = conn();
        let admin_id = super::insert("name", "pwd", &conn)?;
        let npub_value = "npub1test123";
        super::set_npub(admin_id.id, Some(npub_value.to_string()), &conn)?;
        assert_eq!(
            Some(npub_value.to_string()),
            super::select_by_id(admin_id.id, &conn)?.npub
        );
        Ok(())
    }

    #[test]
    fn set_npub_null() -> Result<()> {
        let conn = conn();
        let admin_id = super::insert("name", "pwd", &conn)?;
        super::set_npub(admin_id.id, Some("npub1test123".to_string()), &conn)?;
        super::set_npub(admin_id.id, None, &conn)?;
        assert_eq!(None, super::select_by_id(admin_id.id, &conn)?.npub);
        Ok(())
    }
}
