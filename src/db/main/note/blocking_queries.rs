use super::schema::{self, Columns, Note};
use crate::Result;
use rusqlite::{named_params, params, Connection};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn insert(
    user_id: i64,
    lat: f64,
    lon: f64,
    text: impl Into<String>,
    public: bool,
    conn: &Connection,
) -> Result<Note> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({user_id}, {lat}, {lon}, {text}, {public})
            VALUES (?1, ?2, ?3, ?4, ?5)
            RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        user_id = Columns::UserId.as_ref(),
        lat = Columns::Lat.as_ref(),
        lon = Columns::Lon.as_ref(),
        text = Columns::Text.as_ref(),
        public = Columns::Public.as_ref(),
        projection = Note::projection(),
    );
    conn.query_row(
        &sql,
        params![user_id, lat, lon, text.into(), public],
        Note::mapper(),
    )
    .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<Note> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = Note::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_ref(),
    );
    conn.query_row(&sql, params![id], Note::mapper())
        .map_err(Into::into)
}

/// All of one user's notes, public and private, ordered by `updated_at` so a
/// client can page the change log with `updated_since`.
pub fn select_by_user_id(
    user_id: i64,
    updated_since: &OffsetDateTime,
    include_deleted: bool,
    limit: i64,
    conn: &Connection,
) -> Result<Vec<Note>> {
    let include_deleted_sql = if include_deleted {
        ""
    } else {
        "AND deleted_at IS NULL"
    };
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {user_id} = :user_id
              AND julianday({updated_at}) > julianday(:updated_since)
              {include_deleted_sql}
            ORDER BY {updated_at}, {id}
            LIMIT :limit
        "#,
        projection = Note::projection(),
        table = schema::TABLE_NAME,
        user_id = Columns::UserId.as_ref(),
        updated_at = Columns::UpdatedAt.as_ref(),
        id = Columns::Id.as_ref(),
    );
    conn.prepare(&sql)?
        .query_map(
            named_params! {
                ":user_id": user_id,
                ":updated_since": updated_since.format(&Rfc3339)?,
                ":limit": limit,
            },
            Note::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Public, non-deleted notes inside a bounding box. Used by the "notes around
/// me" search, which never exposes private rows.
pub fn select_public_in_bbox(
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
    limit: i64,
    conn: &Connection,
) -> Result<Vec<Note>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {public} = 1
              AND {deleted_at} IS NULL
              AND {lat} BETWEEN :min_lat AND :max_lat
              AND {lon} BETWEEN :min_lon AND :max_lon
            ORDER BY {updated_at} DESC, {id} DESC
            LIMIT :limit
        "#,
        projection = Note::projection(),
        table = schema::TABLE_NAME,
        public = Columns::Public.as_ref(),
        deleted_at = Columns::DeletedAt.as_ref(),
        lat = Columns::Lat.as_ref(),
        lon = Columns::Lon.as_ref(),
        updated_at = Columns::UpdatedAt.as_ref(),
        id = Columns::Id.as_ref(),
    );
    conn.prepare(&sql)?
        .query_map(
            named_params! {
                ":min_lat": min_lat,
                ":max_lat": max_lat,
                ":min_lon": min_lon,
                ":max_lon": max_lon,
                ":limit": limit,
            },
            Note::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn update(id: i64, text: impl Into<String>, public: bool, conn: &Connection) -> Result<Note> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {text} = ?2, {public} = ?3
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        text = Columns::Text.as_ref(),
        public = Columns::Public.as_ref(),
        id = Columns::Id.as_ref(),
    );
    conn.execute(&sql, params![id, text.into(), public])?;
    select_by_id(id, conn)
}

pub fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    conn: &Connection,
) -> Result<Note> {
    match deleted_at {
        Some(deleted_at) => {
            let sql = format!(
                r#"
                    UPDATE {table}
                    SET {deleted_at} = ?2
                    WHERE {id} = ?1
                "#,
                table = schema::TABLE_NAME,
                deleted_at = Columns::DeletedAt.as_ref(),
                id = Columns::Id.as_ref(),
            );
            conn.execute(&sql, params![id, deleted_at.format(&Rfc3339)?])?;
        }
        None => {
            let sql = format!(
                r#"
                    UPDATE {table}
                    SET {deleted_at} = NULL
                    WHERE {id} = ?1
                "#,
                table = schema::TABLE_NAME,
                deleted_at = Columns::DeletedAt.as_ref(),
                id = Columns::Id.as_ref(),
            );
            conn.execute(&sql, params![id])?;
        }
    };
    select_by_id(id, conn)
}

#[cfg(test)]
mod test {
    use crate::{db::main::test::conn, Result};
    use time::macros::datetime;

    #[test]
    fn insert_and_select_by_id() -> Result<()> {
        let conn = conn();
        let user = crate::db::main::user::blocking_queries::insert("tester", "", &conn)?;
        let inserted = super::insert(user.id, 1.23, 4.56, "hello", true, &conn)?;

        let selected = super::select_by_id(inserted.id, &conn)?;
        assert_eq!(inserted, selected);
        assert_eq!(selected.user_id, user.id);
        assert_eq!(selected.lat, 1.23);
        assert_eq!(selected.lon, 4.56);
        assert_eq!(selected.text, "hello");
        assert!(selected.public);
        assert_eq!(selected.deleted_at, None);
        Ok(())
    }

    #[test]
    fn select_by_user_id_filters_and_pages() -> Result<()> {
        let conn = conn();
        let user = crate::db::main::user::blocking_queries::insert("tester", "", &conn)?;
        let other = crate::db::main::user::blocking_queries::insert("other", "", &conn)?;
        super::insert(user.id, 1.0, 1.0, "mine", false, &conn)?;
        super::insert(other.id, 1.0, 1.0, "theirs", true, &conn)?;

        let mine =
            super::select_by_user_id(user.id, &datetime!(1970-01-01 0:00 UTC), false, 10, &conn)?;
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].text, "mine");
        Ok(())
    }

    #[test]
    fn select_by_user_id_excludes_deleted_unless_requested() -> Result<()> {
        let conn = conn();
        let user = crate::db::main::user::blocking_queries::insert("tester", "", &conn)?;
        let note = super::insert(user.id, 1.0, 1.0, "bye", false, &conn)?;
        super::set_deleted_at(note.id, Some(time::OffsetDateTime::now_utc()), &conn)?;

        let live =
            super::select_by_user_id(user.id, &datetime!(1970-01-01 0:00 UTC), false, 10, &conn)?;
        assert!(live.is_empty());

        let all =
            super::select_by_user_id(user.id, &datetime!(1970-01-01 0:00 UTC), true, 10, &conn)?;
        assert_eq!(all.len(), 1);
        assert!(all[0].deleted_at.is_some());
        Ok(())
    }

    #[test]
    fn select_public_in_bbox_filters_private_deleted_and_out_of_range() -> Result<()> {
        let conn = conn();
        let user = crate::db::main::user::blocking_queries::insert("tester", "", &conn)?;
        super::insert(user.id, 1.0, 1.0, "public", true, &conn)?;
        super::insert(user.id, 1.0, 1.0, "private", false, &conn)?;
        let deleted = super::insert(user.id, 1.0, 1.0, "deleted", true, &conn)?;
        super::set_deleted_at(deleted.id, Some(time::OffsetDateTime::now_utc()), &conn)?;
        super::insert(user.id, 50.0, 50.0, "far", true, &conn)?;

        let hits = super::select_public_in_bbox(0.0, 2.0, 0.0, 2.0, 100, &conn)?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].text, "public");
        Ok(())
    }

    #[test]
    fn update_changes_text_and_visibility() -> Result<()> {
        let conn = conn();
        let user = crate::db::main::user::blocking_queries::insert("tester", "", &conn)?;
        let note = super::insert(user.id, 1.0, 1.0, "old", false, &conn)?;

        let updated = super::update(note.id, "new", true, &conn)?;
        assert_eq!(updated.text, "new");
        assert!(updated.public);
        Ok(())
    }
}
