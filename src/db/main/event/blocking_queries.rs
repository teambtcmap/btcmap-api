use crate::{
    db::main::event::schema::{self, Event},
    Result,
};
use rusqlite::{named_params, params, Connection, ToSql};
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
    cron_schedule: Option<&str>,
    conn: &Connection,
) -> Result<Event> {
    let sql = format!(
        r#"
            INSERT INTO {TABLE} ({AreaId}, {Lat}, {Lon}, {Name}, {Website}, {StartsAt}, {EndsAt}, {CronSchedule})
            VALUES (:area_id, :lat, :lon, :name, :website, :starts_at, :ends_at, :cron_schedule)
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
        ":cron_schedule": cron_schedule,
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

#[allow(clippy::too_many_arguments)]
pub fn update(
    id: i64,
    area_id: Option<Option<i64>>,
    lat: Option<f64>,
    lon: Option<f64>,
    name: Option<&str>,
    website: Option<&str>,
    starts_at: Option<Option<OffsetDateTime>>,
    ends_at: Option<Option<OffsetDateTime>>,
    cron_schedule: Option<Option<&str>>,
    conn: &Connection,
) -> Result<Event> {
    let mut sets: Vec<String> = Vec::new();
    let mut sql_params: Vec<(&str, &dyn ToSql)> = vec![(":id", &id)];

    if let Some(v) = &area_id {
        sets.push(format!("{AreaId} = :area_id"));
        sql_params.push((":area_id", v));
    }
    if let Some(v) = &lat {
        sets.push(format!("{Lat} = :lat"));
        sql_params.push((":lat", v));
    }
    if let Some(v) = &lon {
        sets.push(format!("{Lon} = :lon"));
        sql_params.push((":lon", v));
    }
    if let Some(v) = &name {
        sets.push(format!("{Name} = :name"));
        sql_params.push((":name", v));
    }
    if let Some(v) = &website {
        sets.push(format!("{Website} = :website"));
        sql_params.push((":website", v));
    }
    if let Some(v) = &starts_at {
        sets.push(format!("{StartsAt} = :starts_at"));
        sql_params.push((":starts_at", v));
    }
    if let Some(v) = &ends_at {
        sets.push(format!("{EndsAt} = :ends_at"));
        sql_params.push((":ends_at", v));
    }
    if let Some(v) = &cron_schedule {
        sets.push(format!("{CronSchedule} = :cron_schedule"));
        sql_params.push((":cron_schedule", v));
    }

    if sets.is_empty() {
        return select_by_id(id, conn);
    }

    let sql = format!(
        r#"
            UPDATE {TABLE}
            SET {}
            WHERE {Id} = :id
            RETURNING {projection}
        "#,
        sets.join(", "),
        projection = Event::projection(),
    );
    conn.query_row(&sql, sql_params.as_slice(), Event::mapper())
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
    use crate::{
        db::main::{area::schema::Area, test::conn},
        Result,
    };
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
            Some("0 0 * * * *"),
            &conn,
        )?;
        assert_eq!(Some(&event), super::select_all(&conn)?.first());
        Ok(())
    }

    #[test]
    fn update() -> Result<()> {
        let conn = conn();
        let event = super::insert(
            None,
            1.23,
            4.56,
            "name",
            "website",
            Some(OffsetDateTime::now_utc()),
            None,
            Some("0 0 * * * *"),
            &conn,
        )?;
        let updated = super::update(
            event.id,
            None,
            Some(7.89),
            Some(0.12),
            Some("renamed"),
            Some("https://example.com"),
            Some(None),
            Some(None),
            Some(None),
            &conn,
        )?;
        assert_eq!(updated.id, event.id);
        assert_eq!(updated.lat, 7.89);
        assert_eq!(updated.lon, 0.12);
        assert_eq!(updated.name, "renamed");
        assert_eq!(updated.website, "https://example.com");
        assert!(updated.starts_at.is_none());
        assert!(updated.ends_at.is_none());
        assert!(updated.cron_schedule.is_none());
        assert!(updated.updated_at >= event.updated_at);
        Ok(())
    }

    #[test]
    fn update_partial_only_name() -> Result<()> {
        let conn = conn();
        let event = super::insert(
            None,
            1.23,
            4.56,
            "name",
            "website",
            Some(OffsetDateTime::now_utc()),
            None,
            Some("0 0 * * * *"),
            &conn,
        )?;
        let updated = super::update(
            event.id,
            None,
            None,
            None,
            Some("renamed"),
            None,
            None,
            None,
            None,
            &conn,
        )?;
        assert_eq!(updated.id, event.id);
        assert_eq!(updated.lat, 1.23);
        assert_eq!(updated.lon, 4.56);
        assert_eq!(updated.name, "renamed");
        assert_eq!(updated.website, "website");
        assert_eq!(updated.starts_at, event.starts_at);
        assert_eq!(updated.ends_at, event.ends_at);
        assert_eq!(updated.cron_schedule, event.cron_schedule);
        assert!(updated.updated_at >= event.updated_at);
        Ok(())
    }

    #[test]
    fn update_no_fields_returns_existing() -> Result<()> {
        let conn = conn();
        let event = super::insert(None, 1.23, 4.56, "name", "website", None, None, None, &conn)?;
        let original_updated_at = event.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        let returned = super::update(
            event.id, None, None, None, None, None, None, None, None, &conn,
        )?;
        assert_eq!(returned.id, event.id);
        assert_eq!(returned.name, "name");
        assert_eq!(
            returned.updated_at, original_updated_at,
            "no-op update must not bump updated_at"
        );
        Ok(())
    }

    #[test]
    fn update_area_id() -> Result<()> {
        let conn = conn();
        let area = crate::db::main::area::blocking_queries::insert(Area::mock_tags(), &conn)?;
        let event = super::insert(None, 1.23, 4.56, "name", "website", None, None, None, &conn)?;
        assert_eq!(event.area_id, None);

        let updated = super::update(
            event.id,
            Some(Some(area.id)),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &conn,
        )?;
        assert_eq!(updated.area_id, Some(area.id));

        let updated = super::update(
            event.id,
            Some(None),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &conn,
        )?;
        assert_eq!(updated.area_id, None);

        Ok(())
    }

    #[test]
    fn update_missing_row() {
        let conn = conn();
        let res = super::update(999, None, None, None, None, None, None, None, None, &conn);
        assert!(res.is_err());
    }

    #[test]
    fn insert_null_started_at() -> Result<()> {
        let conn = conn();
        let event = super::insert(None, 1.23, 4.56, "name", "website", None, None, None, &conn)?;
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
