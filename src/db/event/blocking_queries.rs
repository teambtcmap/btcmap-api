use crate::{
    db::event::schema::{self, Columns, Event},
    Result,
};
use rusqlite::{named_params, params, Connection};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn insert(
    lat: f64,
    lon: f64,
    name: &str,
    website: &str,
    starts_at: Option<OffsetDateTime>,
    ends_at: Option<OffsetDateTime>,
    conn: &Connection,
) -> Result<Event> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({lat}, {lon}, {name}, {website}, {starts_at}, {ends_at})
            VALUES (:lat, :lon, :name, :website, :starts_at, :ends_at)
            RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        lat = Columns::Lat.as_str(),
        lon = Columns::Lon.as_str(),
        name = Columns::Name.as_str(),
        website = Columns::Website.as_str(),
        starts_at = Columns::StartsAt.as_str(),
        ends_at = Columns::EndsAt.as_str(),
        projection = Event::projection(),
    );
    let params = named_params! {
        ":lat": lat,
        ":lon": lon,
        ":name": name,
        ":website": website,
        ":starts_at": starts_at,
        ":ends_at": ends_at,
    };
    conn.query_row(&sql, params, Event::mapper())
        .map_err(Into::into)
}

pub fn select_all(conn: &Connection) -> Result<Vec<Event>> {
    let sql = format!(
        "
        SELECT {projection}
        FROM {table}
    ",
        projection = Event::projection(),
        table = schema::TABLE_NAME
    );
    conn.prepare(&sql)?
        .query_map({}, Event::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<Event> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = Event::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![id], Event::mapper())
        .map_err(Into::into)
}

pub fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    conn: &Connection,
) -> Result<Event> {
    match deleted_at {
        Some(deleted_at) => {
            let sql = format!(
                r#"
                    UPDATE {table}
                    SET {deleted_at} = ?2
                    WHERE {id} = ?1
                    RETURNING {projection}
                "#,
                table = schema::TABLE_NAME,
                deleted_at = Columns::DeletedAt.as_str(),
                id = Columns::Id.as_str(),
                projection = Event::projection(),
            );
            conn.query_row(
                &sql,
                params![id, deleted_at.format(&Rfc3339)?],
                Event::mapper(),
            )
            .map_err(Into::into)
        }
        None => {
            let sql = format!(
                r#"
                    UPDATE {table}
                    SET {deleted_at} = NULL
                    WHERE {id} = ?1
                    RETURNING {projection}
                "#,
                table = schema::TABLE_NAME,
                deleted_at = Columns::DeletedAt.as_str(),
                id = Columns::Id.as_str(),
                projection = Event::projection(),
            );
            conn.query_row(&sql, params![id], Event::mapper())
                .map_err(Into::into)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{db::test::conn, Result};
    use time::OffsetDateTime;

    #[test]
    fn insert() -> Result<()> {
        let conn = conn();
        let event = super::insert(
            1.23,
            4.56,
            "name",
            "website",
            Some(OffsetDateTime::now_utc()),
            None,
            &conn,
        )?;
        assert_eq!(Some(&event), super::select_all(&conn)?.first());
        Ok(())
    }

    #[test]
    fn insert_null_started_at() -> Result<()> {
        let conn = conn();
        let event = super::insert(1.23, 4.56, "name", "website", None, None, &conn)?;
        assert_eq!(Some(&event), super::select_all(&conn)?.first());
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = conn();
        let event_1 = super::insert(
            1.23,
            4.56,
            "name",
            "website",
            Some(OffsetDateTime::now_utc()),
            None,
            &conn,
        )?;
        let event_2 = super::insert(
            1.23,
            4.56,
            "name",
            "website",
            Some(OffsetDateTime::now_utc()),
            None,
            &conn,
        )?;
        let event_3 = super::insert(
            1.23,
            4.56,
            "name",
            "website",
            Some(OffsetDateTime::now_utc()),
            None,
            &conn,
        )?;
        assert_eq!(vec![event_1, event_2, event_3], super::select_all(&conn)?);
        Ok(())
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = conn();
        let event = super::insert(
            1.23,
            4.56,
            "name",
            "website",
            Some(OffsetDateTime::now_utc()),
            None,
            &conn,
        )?;
        let event = super::set_deleted_at(event.id, Some(OffsetDateTime::now_utc()), &conn)?;
        assert!(event.deleted_at.is_some());
        assert!(super::select_all(&conn)?
            .first()
            .map(|it| it.deleted_at)
            .is_some());
        let event = super::set_deleted_at(event.id, None, &conn)?;
        assert!(event.deleted_at.is_none());
        assert_eq!(
            Some(None),
            super::select_all(&conn)?.first().map(|it| it.deleted_at)
        );
        Ok(())
    }
}
