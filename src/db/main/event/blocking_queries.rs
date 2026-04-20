use crate::{
    db::main::event::schema::{self, Event},
    Result,
};
use rusqlite::{named_params, params, Connection};
use schema::Columns::*;
use schema::TABLE;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[allow(clippy::too_many_arguments)]
pub fn insert(
    area_id: Option<i64>,
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
            INSERT INTO {TABLE} ({AreaId}, {Lat}, {Lon}, {Name}, {Website}, {StartsAt}, {EndsAt})
            VALUES (:area_id, :lat, :lon, :name, :website, :starts_at, :ends_at)
            RETURNING {projection}
        "#,
        projection = Event::projection(),
    );
    let params = named_params! {
        ":area_id": area_id,
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
        FROM {TABLE}
    ",
        projection = Event::projection(),
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
            FROM {TABLE}
            WHERE {Id} = ?1
        "#,
        projection = Event::projection(),
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
                    UPDATE {TABLE}
                    SET {DeletedAt} = ?2
                    WHERE {Id} = ?1
                    RETURNING {projection}
                "#,
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
                    UPDATE {TABLE}
                    SET {DeletedAt} = NULL
                    WHERE {Id} = ?1
                    RETURNING {projection}
                "#,
                projection = Event::projection(),
            );
            conn.query_row(&sql, params![id], Event::mapper())
                .map_err(Into::into)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{db::main::test::conn, Result};
    use time::OffsetDateTime;

    #[test]
    fn insert() -> Result<()> {
        let conn = conn();
        let event = super::insert(
            None,
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
        let event = super::insert(None, 1.23, 4.56, "name", "website", None, None, &conn)?;
        assert_eq!(Some(&event), super::select_all(&conn)?.first());
        Ok(())
    }

    #[test]
    fn select_all() -> Result<()> {
        let conn = conn();
        let event_1 = super::insert(
            None,
            1.23,
            4.56,
            "name",
            "website",
            Some(OffsetDateTime::now_utc()),
            None,
            &conn,
        )?;
        let event_2 = super::insert(
            None,
            1.23,
            4.56,
            "name",
            "website",
            Some(OffsetDateTime::now_utc()),
            None,
            &conn,
        )?;
        let event_3 = super::insert(
            None,
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
            None,
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
