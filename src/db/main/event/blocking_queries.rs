use crate::service::search::{escape_like, split_words};
use crate::{
    db::main::event::schema::{self, Event},
    Result,
};
use rusqlite::params_from_iter;
use rusqlite::types::Value as SqlValue;
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
    starts_at: OffsetDateTime,
    ends_at: Option<OffsetDateTime>,
    conn: &Connection,
) -> Result<Event> {
    insert_with_status(
        area_id,
        lat,
        lon,
        name,
        website,
        starts_at,
        ends_at,
        schema::Status::Live,
        None,
        conn,
    )
}

/// Same as [`insert`] but lets the caller pick the initial review [`Status`]
/// and the submitting user. Existing callers (RPC and tests) create published,
/// unattributed events; the v4 submission endpoint uses this to create
/// `pending` events owned by the signed-in user.
#[allow(clippy::too_many_arguments)]
pub fn insert_with_status(
    area_id: Option<i64>,
    lat: f64,
    lon: f64,
    name: &str,
    website: &str,
    starts_at: OffsetDateTime,
    ends_at: Option<OffsetDateTime>,
    status: schema::Status,
    submitted_by: Option<i64>,
    conn: &Connection,
) -> Result<Event> {
    let sql = format!(
        r#"
            INSERT INTO {TABLE} ({AreaId}, {Lat}, {Lon}, {Name}, {Website}, {StartsAt}, {EndsAt}, {Status}, {SubmittedBy})
            VALUES (:area_id, :lat, :lon, :name, :website, :starts_at, :ends_at, :status, :submitted_by)
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
        ":status": status,
        ":submitted_by": submitted_by,
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
    starts_at: Option<OffsetDateTime>,
    ends_at: Option<Option<OffsetDateTime>>,
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

/// Every non-deleted event submitted by `user_id`, newest first. Feeds
/// `GET /v4/users/me/events` so a submitter can track review status.
pub fn select_by_submitted_by(user_id: i64, conn: &Connection) -> Result<Vec<Event>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {TABLE}
            WHERE {SubmittedBy} = ?1
              AND {DeletedAt} IS NULL
            ORDER BY {CreatedAt} DESC, {Id} DESC
        "#,
        projection = Event::projection(),
    );
    conn.prepare(&sql)?
        .query_map(params![user_id], Event::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// SQL predicate restricting a query to the supplied [`schema::Status`]es.
///
/// The literals are rendered from the enum itself, never from request input,
/// so embedding them in the statement is safe. An empty slice matches nothing,
/// which is deliberate: callers that want no rows must say so explicitly
/// instead of accidentally widening to "all".
fn statuses_predicate(statuses: &[schema::Status]) -> String {
    if statuses.is_empty() {
        return "0".to_string();
    }
    let list = statuses
        .iter()
        .map(|status| format!("'{}'", status.as_str()))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{Status} IN ({list})")
}

/// Delta query for sync clients: every row whose `updated_at` is strictly after
/// `updated_since`, ordered so a client can page through with a timestamp
/// cursor (the caller widens the window when a full page shares one timestamp,
/// exactly like the element and comment delta queries).
///
/// Unlike the full snapshot in [`select_all`], this deliberately does not drop
/// past events: a change log must still surface edits and deletions of events
/// that have already started. Soft-deleted rows are returned only when
/// `include_deleted` is set, so a client can apply tombstones. Only rows whose
/// status is in `statuses` are returned, so the default (live-only) sync client
/// never observes unreviewed submissions, and the `LIMIT` is applied to the
/// matching rows rather than after filtering.
pub fn select_updated_since(
    updated_since: &OffsetDateTime,
    include_deleted: bool,
    statuses: &[schema::Status],
    limit: Option<i64>,
    conn: &Connection,
) -> Result<Vec<Event>> {
    let deleted_filter = if include_deleted {
        ""
    } else {
        "AND deleted_at IS NULL"
    };
    let status_filter = statuses_predicate(statuses);
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {TABLE}
            WHERE julianday({UpdatedAt}) > julianday(:updated_since)
              {deleted_filter}
              AND {status_filter}
            ORDER BY {UpdatedAt}, {Id}
            LIMIT :limit
        "#,
        projection = Event::projection(),
    );
    conn.prepare(&sql)?
        .query_map(
            named_params! {
                ":updated_since": updated_since.format(&Rfc3339)?,
                ":limit": limit.unwrap_or(i64::MAX),
            },
            Event::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Cheap bbox pre-filter: events whose (lat, lon) point falls inside the
/// supplied bounding box. The caller is still responsible for the precise
/// geojson contains check on the returned candidates.
pub fn select_by_bbox(
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    conn: &Connection,
) -> Result<Vec<Event>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {TABLE}
            WHERE {DeletedAt} IS NULL
              AND {Lat} >= ?2
              AND {Lat} <= ?4
              AND {Lon} >= ?1
              AND {Lon} <= ?3
        "#,
        projection = Event::projection(),
    );
    conn.prepare(&sql)?
        .query_map(params![west, south, east, north], Event::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Bbox pre-filter for upcoming events. Drops rows whose `starts_at` is in
/// the past so callers can use the result as a final list without re-checking
/// timestamps, and returns only `live` events so area pages never publish
/// unreviewed submissions. The caller is still responsible for the precise
/// geojson contains check on the returned candidates.
pub fn select_upcoming_by_bbox(
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    conn: &Connection,
) -> Result<Vec<Event>> {
    let now = OffsetDateTime::now_utc().format(&Rfc3339)?;
    let live = schema::Status::Live.as_str();
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {TABLE}
            WHERE {DeletedAt} IS NULL
              AND {StartsAt} >= ?5
              AND {Status} = '{live}'
              AND {Lat} >= ?2
              AND {Lat} <= ?4
              AND {Lon} >= ?1
              AND {Lon} <= ?3
        "#,
        projection = Event::projection(),
    );
    conn.prepare(&sql)?
        .query_map(params![west, south, east, north, now], Event::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

#[derive(Debug, PartialEq)]
pub struct RankedEvent {
    pub event: Event,
    pub rank: i64,
}

/// Matches `query` against the event name only. Every whitespace word must
/// match the name, soft-deleted events are dropped, only future events survive
/// and only `live` events are eligible, mirroring the public `GET /v4/events`
/// default. `starts_at` is the RFC 3339 `TEXT` column, compared
/// lexicographically like [`select_upcoming_by_bbox`].
fn search_predicate(word_count: usize, now_param: usize, first_word_param: usize) -> String {
    let mut words = String::new();
    for i in 0..word_count {
        let param = first_word_param + i;
        words.push_str(&format!(
            r#"
            AND {Name} LIKE ?{param} ESCAPE '\'"#
        ));
    }
    let live = schema::Status::Live.as_str();
    format!(
        "{DeletedAt} IS NULL
         AND {StartsAt} >= ?{now_param}
         AND {Status} = '{live}'{words}"
    )
}

fn word_patterns(words: &[String]) -> impl Iterator<Item = SqlValue> + '_ {
    words
        .iter()
        .map(|word| SqlValue::Text(format!("%{}%", escape_like(word))))
}

/// Ranked name search over future or undated `live` events. `location` breaks
/// rank ties by proximity, exactly as the element and area searches do.
pub fn select_by_search(
    query: &str,
    location: Option<(f64, f64)>,
    row_limit: i64,
    conn: &Connection,
) -> Result<Vec<RankedEvent>> {
    let words = split_words(query);
    let first_word_param = 8;
    let limit_param = first_word_param + words.len();
    let sql = format!(
        r#"
            SELECT {projection},
              CASE
                WHEN {Name} = ?1 COLLATE NOCASE THEN 0
                WHEN {Name} LIKE ?2 ESCAPE '\' THEN 1
                WHEN {Name} LIKE ?3 ESCAPE '\' THEN 2
                ELSE 3
              END AS search_rank
            FROM {TABLE}
            WHERE {predicate}
            ORDER BY search_rank,
                     CASE WHEN ?6 = 1
                          THEN ({Lat} - ?4) * ({Lat} - ?4) + ({Lon} - ?5) * ({Lon} - ?5)
                          ELSE 0 END,
                     LENGTH({Name}),
                     {Name},
                     {Id}
            LIMIT ?{limit_param}
        "#,
        projection = Event::projection(),
        predicate = search_predicate(words.len(), 7, first_word_param),
    );

    let escaped = escape_like(query);
    let (lat, lon, has_location) = match location {
        Some((lat, lon)) => (lat, lon, 1),
        None => (0.0, 0.0, 0),
    };
    let now = OffsetDateTime::now_utc().format(&Rfc3339)?;
    let mut values = vec![
        SqlValue::Text(query.to_string()),
        SqlValue::Text(format!("{escaped}%")),
        SqlValue::Text(format!("%{escaped}%")),
        SqlValue::Real(lat),
        SqlValue::Real(lon),
        SqlValue::Integer(has_location),
        SqlValue::Text(now),
    ];
    values.extend(word_patterns(&words));
    values.push(SqlValue::Integer(row_limit));

    conn.prepare(&sql)?
        .query_map(params_from_iter(values), |row| {
            Ok(RankedEvent {
                event: Event::mapper()(row)?,
                rank: row.get("search_rank")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn count_by_search(query: &str, conn: &Connection) -> Result<i64> {
    let words = split_words(query);
    let sql = format!(
        "SELECT COUNT(*) FROM {TABLE} WHERE {predicate}",
        predicate = search_predicate(words.len(), 1, 2),
    );
    let now = OffsetDateTime::now_utc().format(&Rfc3339)?;
    let mut values = vec![SqlValue::Text(now)];
    values.extend(word_patterns(&words));
    conn.query_row(&sql, params_from_iter(values), |row| row.get(0))
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

pub fn set_status(id: i64, status: schema::Status, conn: &Connection) -> Result<Event> {
    let sql = format!(
        r#"
            UPDATE {TABLE}
            SET {Status} = :status
            WHERE {Id} = :id
            RETURNING {projection}
        "#,
        projection = Event::projection(),
    );
    conn.query_row(
        &sql,
        named_params! { ":id": id, ":status": status },
        Event::mapper(),
    )
    .map_err(Into::into)
}

#[cfg(test)]
mod test {
    use crate::{
        db::main::{area::schema::Area, event::schema::Status, test::conn},
        Result,
    };
    use rusqlite::params;
    use rusqlite::Connection;
    use time::format_description::well_known::Rfc3339;
    use time::macros::datetime;
    use time::Duration;
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
            OffsetDateTime::now_utc(),
            None,
            &conn,
        )?;
        assert_eq!(event.status, Status::Live);
        assert_eq!(Some(&event), super::select_all(&conn)?.first());
        Ok(())
    }

    #[test]
    fn insert_with_status_persists_status() -> Result<()> {
        let conn = conn();
        let event = super::insert_with_status(
            None,
            1.23,
            4.56,
            "name",
            "website",
            OffsetDateTime::now_utc(),
            None,
            Status::Pending,
            Some(42),
            &conn,
        )?;
        assert_eq!(event.status, Status::Pending);
        assert_eq!(event.submitted_by, Some(42));
        assert_eq!(
            Status::Pending,
            super::select_by_id(event.id, &conn)?.status
        );
        Ok(())
    }

    #[test]
    fn select_by_submitted_by_returns_only_that_users_live_rows() -> Result<()> {
        let conn = conn();
        let mine = super::insert_with_status(
            None,
            1.0,
            1.0,
            "mine",
            "website",
            OffsetDateTime::now_utc(),
            None,
            Status::Pending,
            Some(7),
            &conn,
        )?;
        super::insert_with_status(
            None,
            2.0,
            2.0,
            "theirs",
            "website",
            OffsetDateTime::now_utc(),
            None,
            Status::Live,
            Some(8),
            &conn,
        )?;
        super::insert(
            None,
            3.0,
            3.0,
            "unattributed",
            "website",
            OffsetDateTime::now_utc(),
            None,
            &conn,
        )?;

        let mine_only = super::select_by_submitted_by(7, &conn)?;
        assert_eq!(vec![mine], mine_only);
        Ok(())
    }

    #[test]
    fn select_by_submitted_by_excludes_deleted() -> Result<()> {
        let conn = conn();
        let event = super::insert_with_status(
            None,
            1.0,
            1.0,
            "mine",
            "website",
            OffsetDateTime::now_utc(),
            None,
            Status::Live,
            Some(7),
            &conn,
        )?;
        super::set_deleted_at(event.id, Some(OffsetDateTime::now_utc()), &conn)?;
        assert!(super::select_by_submitted_by(7, &conn)?.is_empty());
        Ok(())
    }

    #[test]
    fn set_status_transitions_and_bumps_updated_at() -> Result<()> {
        let conn = conn();
        let event = super::insert(
            None,
            1.23,
            4.56,
            "name",
            "website",
            OffsetDateTime::now_utc(),
            None,
            &conn,
        )?;
        let updated = super::set_status(event.id, Status::Rejected, &conn)?;
        assert_eq!(updated.status, Status::Rejected);
        assert!(updated.updated_at >= event.updated_at);
        assert_eq!(
            Status::Rejected,
            super::select_by_id(event.id, &conn)?.status
        );
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
            OffsetDateTime::now_utc(),
            None,
            &conn,
        )?;
        let updated = super::update(
            event.id,
            None,
            Some(7.89),
            Some(0.12),
            Some("renamed"),
            Some("https://example.com"),
            None,
            Some(None),
            &conn,
        )?;
        assert_eq!(updated.id, event.id);
        assert_eq!(updated.lat, 7.89);
        assert_eq!(updated.lon, 0.12);
        assert_eq!(updated.name, "renamed");
        assert_eq!(updated.website, "https://example.com");
        assert_eq!(updated.starts_at, event.starts_at);
        assert!(updated.ends_at.is_none());
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
            OffsetDateTime::now_utc(),
            None,
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
            &conn,
        )?;
        assert_eq!(updated.id, event.id);
        assert_eq!(updated.lat, 1.23);
        assert_eq!(updated.lon, 4.56);
        assert_eq!(updated.name, "renamed");
        assert_eq!(updated.website, "website");
        assert_eq!(updated.starts_at, event.starts_at);
        assert_eq!(updated.ends_at, event.ends_at);
        assert!(updated.updated_at >= event.updated_at);
        Ok(())
    }

    #[test]
    fn update_no_fields_returns_existing() -> Result<()> {
        let conn = conn();
        let event = super::insert(
            None,
            1.23,
            4.56,
            "name",
            "website",
            OffsetDateTime::now_utc(),
            None,
            &conn,
        )?;
        let original_updated_at = event.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        let returned = super::update(event.id, None, None, None, None, None, None, None, &conn)?;
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
        let event = super::insert(
            None,
            1.23,
            4.56,
            "name",
            "website",
            OffsetDateTime::now_utc(),
            None,
            &conn,
        )?;
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
            &conn,
        )?;
        assert_eq!(updated.area_id, None);

        Ok(())
    }

    #[test]
    fn update_missing_row() {
        let conn = conn();
        let res = super::update(999, None, None, None, None, None, None, None, &conn);
        assert!(res.is_err());
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
            OffsetDateTime::now_utc(),
            None,
            &conn,
        )?;
        let event_2 = super::insert(
            None,
            1.23,
            4.56,
            "name",
            "website",
            OffsetDateTime::now_utc(),
            None,
            &conn,
        )?;
        let event_3 = super::insert(
            None,
            1.23,
            4.56,
            "name",
            "website",
            OffsetDateTime::now_utc(),
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
            OffsetDateTime::now_utc(),
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

    #[test]
    fn select_by_bbox_returns_in_bounds_events() -> Result<()> {
        let conn = conn();
        let inside = super::insert(
            None,
            7.97,
            98.33,
            "inside",
            "website",
            OffsetDateTime::now_utc(),
            None,
            &conn,
        )?;
        super::insert(
            None,
            50.0,
            1.0,
            "outside",
            "website",
            OffsetDateTime::now_utc(),
            None,
            &conn,
        )?;
        let hits = super::select_by_bbox(98.0, 7.0, 99.0, 8.0, &conn)?;
        assert_eq!(vec![inside], hits);
        Ok(())
    }

    #[test]
    fn select_by_bbox_excludes_deleted() -> Result<()> {
        let conn = conn();
        let event = super::insert(
            None,
            7.97,
            98.33,
            "deleted",
            "website",
            OffsetDateTime::now_utc(),
            None,
            &conn,
        )?;
        super::set_deleted_at(event.id, Some(OffsetDateTime::now_utc()), &conn)?;
        let hits = super::select_by_bbox(98.0, 7.0, 99.0, 8.0, &conn)?;
        assert!(hits.is_empty());
        Ok(())
    }

    #[test]
    fn select_by_bbox_is_inclusive_on_edges() -> Result<()> {
        let conn = conn();
        let event = super::insert(
            None,
            8.0,
            98.0,
            "on_edge",
            "website",
            OffsetDateTime::now_utc(),
            None,
            &conn,
        )?;
        let hits = super::select_by_bbox(98.0, 7.0, 99.0, 8.0, &conn)?;
        assert_eq!(vec![event], hits);
        Ok(())
    }

    fn insert_named(
        name: &str,
        starts_at: OffsetDateTime,
        lat: f64,
        lon: f64,
        conn: &Connection,
    ) -> Result<i64> {
        Ok(super::insert(None, lat, lon, name, "website", starts_at, None, conn)?.id)
    }

    fn future_start() -> OffsetDateTime {
        datetime!(2999-01-01 0:00 UTC)
    }

    #[test]
    fn search_ranks_exact_above_prefix_above_infix() -> Result<()> {
        let conn = conn();
        insert_named("Bitcoin", future_start(), 0.0, 0.0, &conn)?;
        insert_named("Bitcoin Meetup", future_start(), 0.0, 0.0, &conn)?;
        insert_named("Meetup Bitcoin", future_start(), 0.0, 0.0, &conn)?;

        let ranked = super::select_by_search("Bitcoin", None, 100, &conn)?;

        assert_eq!(
            vec![0, 1, 2],
            ranked.iter().map(|it| it.rank).collect::<Vec<_>>()
        );
        assert_eq!(
            vec!["Bitcoin", "Bitcoin Meetup", "Meetup Bitcoin"],
            ranked
                .iter()
                .map(|it| it.event.name.as_str())
                .collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn search_breaks_rank_ties_by_distance() -> Result<()> {
        let conn = conn();
        let far = insert_named("Bitcoin Meetup", future_start(), 10.0, 0.0, &conn)?;
        let near = insert_named("Bitcoin Meetup", future_start(), 0.1, 0.0, &conn)?;

        let ranked = super::select_by_search("Bitcoin", Some((0.0, 0.0)), 100, &conn)?;

        assert_eq!(
            vec![near, far],
            ranked.iter().map(|it| it.event.id).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn search_is_case_insensitive() -> Result<()> {
        let conn = conn();
        insert_named("Bitcoin Meetup", future_start(), 0.0, 0.0, &conn)?;

        assert_eq!(
            1,
            super::select_by_search("bItCoIn", None, 100, &conn)?.len()
        );
        Ok(())
    }

    #[test]
    fn search_requires_every_word() -> Result<()> {
        let conn = conn();
        insert_named("Bitcoin Paris Meetup", future_start(), 0.0, 0.0, &conn)?;
        insert_named("Bitcoin Berlin Meetup", future_start(), 0.0, 0.0, &conn)?;

        let ranked = super::select_by_search("bitcoin paris", None, 100, &conn)?;

        assert_eq!(1, ranked.len());
        assert_eq!("Bitcoin Paris Meetup", ranked[0].event.name);
        Ok(())
    }

    #[test]
    fn search_excludes_deleted() -> Result<()> {
        let conn = conn();
        let event = insert_named("Bitcoin Meetup", future_start(), 0.0, 0.0, &conn)?;
        super::set_deleted_at(event, Some(OffsetDateTime::now_utc()), &conn)?;

        assert!(super::select_by_search("Bitcoin", None, 100, &conn)?.is_empty());
        assert_eq!(0, super::count_by_search("Bitcoin", &conn)?);
        Ok(())
    }

    #[test]
    fn search_excludes_past() -> Result<()> {
        let conn = conn();
        insert_named(
            "Event Past",
            datetime!(2020-01-01 0:00 UTC),
            0.0,
            0.0,
            &conn,
        )?;
        let future = insert_named("Event Future", future_start(), 0.0, 0.0, &conn)?;

        let ids = super::select_by_search("Event", None, 100, &conn)?
            .into_iter()
            .map(|it| it.event.id)
            .collect::<Vec<_>>();

        assert_eq!(vec![future], ids);
        Ok(())
    }

    #[test]
    fn search_respects_row_limit() -> Result<()> {
        let conn = conn();
        for name in ["Bitcoin One", "Bitcoin Two", "Bitcoin Three"] {
            insert_named(name, future_start(), 0.0, 0.0, &conn)?;
        }

        assert_eq!(2, super::select_by_search("Bitcoin", None, 2, &conn)?.len());
        Ok(())
    }

    #[test]
    fn search_escapes_like_wildcards() -> Result<()> {
        let conn = conn();
        insert_named("Bitcoin Meetup", future_start(), 0.0, 0.0, &conn)?;

        assert!(super::select_by_search("%", None, 100, &conn)?.is_empty());
        assert_eq!(0, super::count_by_search("%", &conn)?);
        Ok(())
    }

    #[test]
    fn count_by_search_counts_all_matches() -> Result<()> {
        let conn = conn();
        for name in ["Bitcoin One", "Bitcoin Two", "Bitcoin Three", "Ethereum"] {
            insert_named(name, future_start(), 0.0, 0.0, &conn)?;
        }

        assert_eq!(3, super::count_by_search("Bitcoin", &conn)?);
        Ok(())
    }

    /// Pins `updated_at` directly so cursor tests don't depend on wall-clock
    /// resolution between inserts. The `event_updated_at` trigger only fires on
    /// other columns, so this does not recurse.
    fn set_updated_at(id: i64, updated_at: OffsetDateTime, conn: &Connection) -> Result<()> {
        conn.execute(
            "UPDATE event SET updated_at = ?2 WHERE id = ?1",
            params![id, updated_at.format(&Rfc3339)?],
        )?;
        Ok(())
    }

    #[test]
    fn select_updated_since_filters_by_cursor() -> Result<()> {
        let conn = conn();
        let now = OffsetDateTime::now_utc();
        let old = super::insert(
            None,
            1.0,
            1.0,
            "old",
            "website",
            datetime!(2999-01-01 0:00 UTC),
            None,
            &conn,
        )?;
        set_updated_at(old.id, now - Duration::hours(1), &conn)?;
        let new = super::insert(
            None,
            2.0,
            2.0,
            "new",
            "website",
            datetime!(2999-01-01 0:00 UTC),
            None,
            &conn,
        )?;
        set_updated_at(new.id, now + Duration::hours(1), &conn)?;

        let results = super::select_updated_since(&now, false, &[Status::Live], None, &conn)?;
        assert_eq!(
            vec![new.id],
            results.into_iter().map(|it| it.id).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn select_updated_since_compares_by_instant_not_text() -> Result<()> {
        let conn = conn();
        let event = super::insert(
            None,
            1.0,
            1.0,
            "name",
            "website",
            datetime!(2999-01-01 0:00 UTC),
            None,
            &conn,
        )?;
        conn.execute(
            "UPDATE event SET updated_at = '2024-01-01T10:00:00.550Z' WHERE id = ?1",
            params![event.id],
        )?;

        // The bound is rendered by `time` as "...10:00:00.5Z" (trailing zero
        // trimmed), which sorts before ".550Z" as text. The cursor is compared
        // by instant, so the row must still be returned.
        let results = super::select_updated_since(
            &datetime!(2024-01-01 10:00:00.500 UTC),
            false,
            &[Status::Live],
            None,
            &conn,
        )?;

        assert_eq!(1, results.len());
        assert_eq!(event.id, results[0].id);
        Ok(())
    }

    #[test]
    fn select_updated_since_deleted_rows_only_when_included() -> Result<()> {
        let conn = conn();
        let event = super::insert(
            None,
            1.0,
            1.0,
            "name",
            "website",
            datetime!(2999-01-01 0:00 UTC),
            None,
            &conn,
        )?;
        super::set_deleted_at(event.id, Some(OffsetDateTime::now_utc()), &conn)?;

        let without = super::select_updated_since(
            &OffsetDateTime::UNIX_EPOCH,
            false,
            &[Status::Live],
            None,
            &conn,
        )?;
        assert!(without.is_empty());

        let included = super::select_updated_since(
            &OffsetDateTime::UNIX_EPOCH,
            true,
            &[Status::Live],
            None,
            &conn,
        )?;
        assert_eq!(1, included.len());
        assert!(included[0].deleted_at.is_some());
        Ok(())
    }

    #[test]
    fn select_updated_since_respects_limit_and_orders_by_updated_at() -> Result<()> {
        let conn = conn();
        let base = OffsetDateTime::UNIX_EPOCH + Duration::days(1);
        let first = super::insert(
            None,
            1.0,
            1.0,
            "first",
            "website",
            datetime!(2999-01-01 0:00 UTC),
            None,
            &conn,
        )?;
        set_updated_at(first.id, base, &conn)?;
        let second = super::insert(
            None,
            2.0,
            2.0,
            "second",
            "website",
            datetime!(2999-01-01 0:00 UTC),
            None,
            &conn,
        )?;
        set_updated_at(second.id, base + Duration::hours(1), &conn)?;
        let third = super::insert(
            None,
            3.0,
            3.0,
            "third",
            "website",
            datetime!(2999-01-01 0:00 UTC),
            None,
            &conn,
        )?;
        set_updated_at(third.id, base + Duration::hours(2), &conn)?;

        let page = super::select_updated_since(
            &OffsetDateTime::UNIX_EPOCH,
            false,
            &[Status::Live],
            Some(2),
            &conn,
        )?;
        assert_eq!(
            vec![first.id, second.id],
            page.into_iter().map(|it| it.id).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn select_updated_since_includes_past_events() -> Result<()> {
        let conn = conn();
        let past = super::insert(
            None,
            1.0,
            1.0,
            "past",
            "website",
            datetime!(2020-01-01 0:00 UTC),
            None,
            &conn,
        )?;

        let results = super::select_updated_since(
            &OffsetDateTime::UNIX_EPOCH,
            false,
            &[Status::Live],
            None,
            &conn,
        )?;
        assert_eq!(
            vec![past.id],
            results.into_iter().map(|it| it.id).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn select_updated_since_respects_status_filter() -> Result<()> {
        let conn = conn();
        let live = super::insert(
            None,
            1.0,
            1.0,
            "live",
            "website",
            future_start(),
            None,
            &conn,
        )?;
        let pending = super::insert_with_status(
            None,
            2.0,
            2.0,
            "pending",
            "website",
            future_start(),
            None,
            Status::Pending,
            None,
            &conn,
        )?;
        set_updated_at(
            live.id,
            OffsetDateTime::UNIX_EPOCH + Duration::days(1),
            &conn,
        )?;
        set_updated_at(
            pending.id,
            OffsetDateTime::UNIX_EPOCH + Duration::days(2),
            &conn,
        )?;

        let live_only = super::select_updated_since(
            &OffsetDateTime::UNIX_EPOCH,
            false,
            &[Status::Live],
            None,
            &conn,
        )?;
        assert_eq!(
            vec![live.id],
            live_only.into_iter().map(|it| it.id).collect::<Vec<_>>()
        );

        let all = super::select_updated_since(
            &OffsetDateTime::UNIX_EPOCH,
            false,
            &Status::ALL,
            None,
            &conn,
        )?;
        assert_eq!(
            vec![live.id, pending.id],
            all.into_iter().map(|it| it.id).collect::<Vec<_>>()
        );

        let none =
            super::select_updated_since(&OffsetDateTime::UNIX_EPOCH, false, &[], None, &conn)?;
        assert!(none.is_empty());
        Ok(())
    }

    #[test]
    fn select_upcoming_by_bbox_excludes_non_live_events() -> Result<()> {
        let conn = conn();
        let live = super::insert(
            None,
            7.97,
            98.33,
            "live",
            "website",
            future_start(),
            None,
            &conn,
        )?;
        super::insert_with_status(
            None,
            7.97,
            98.33,
            "pending",
            "website",
            future_start(),
            None,
            Status::Pending,
            None,
            &conn,
        )?;

        let hits = super::select_upcoming_by_bbox(98.0, 7.0, 99.0, 8.0, &conn)?;
        assert_eq!(vec![live], hits);
        Ok(())
    }

    #[test]
    fn search_excludes_non_live_events() -> Result<()> {
        let conn = conn();
        let live = insert_named("Bitcoin Live", future_start(), 0.0, 0.0, &conn)?;
        super::insert_with_status(
            None,
            0.0,
            0.0,
            "Bitcoin Pending",
            "website",
            future_start(),
            None,
            Status::Pending,
            None,
            &conn,
        )?;
        super::insert_with_status(
            None,
            0.0,
            0.0,
            "Bitcoin Rejected",
            "website",
            future_start(),
            None,
            Status::Rejected,
            None,
            &conn,
        )?;

        let ranked = super::select_by_search("Bitcoin", None, 100, &conn)?;
        assert_eq!(
            vec![live],
            ranked.into_iter().map(|it| it.event.id).collect::<Vec<_>>()
        );
        assert_eq!(1, super::count_by_search("Bitcoin", &conn)?);
        Ok(())
    }
}
