use super::schema::{self, Columns, OriginSubmissionCounts, PlaceSubmission};
use crate::Result;
use geojson::JsonObject;
use rusqlite::{named_params, params, Connection, OptionalExtension};
use serde_json::{Map, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub struct InsertArgs {
    pub origin: String,
    pub external_id: String,
    pub lat: f64,
    pub lon: f64,
    pub category: String,
    pub name: String,
    pub extra_fields: Map<String, Value>,
}

pub fn insert(args: &InsertArgs, conn: &Connection) -> Result<PlaceSubmission> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({origin}, {external_id}, {lat}, {lon}, {category}, {name}, {extra_fields}) 
            VALUES (:origin, :external_id, :lat, :lon, :category, :name, json(:extra_fields))
            RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        origin = Columns::Origin.as_str(),
        external_id = Columns::ExternalId.as_str(),
        lat = Columns::Lat.as_str(),
        lon = Columns::Lon.as_str(),
        category = Columns::Category.as_str(),
        name = Columns::Name.as_str(),
        extra_fields = Columns::ExtraFields.as_str(),
        projection = PlaceSubmission::projection(),
    );
    conn.query_row(
        &sql,
        named_params! {
            ":origin": &args.origin,
            ":external_id": &args.external_id,
            ":lat": args.lat,
            ":lon": args.lon,
            ":category": &args.category,
            ":name": &args.name,
            ":extra_fields": serde_json::to_string(&args.extra_fields)?,

        },
        PlaceSubmission::mapper(),
    )
    .map_err(Into::into)
}

pub fn select_open_and_not_revoked(conn: &Connection) -> Result<Vec<PlaceSubmission>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {closed_at} IS NULL AND {revoked} = 0
            ORDER BY {updated_at} DESC, {id} DESC
        "#,
        projection = PlaceSubmission::projection(),
        table = schema::TABLE_NAME,
        closed_at = Columns::ClosedAt.as_str(),
        revoked = Columns::Revoked.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(params![], PlaceSubmission::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_revoked_with_ticket_url(conn: &Connection) -> Result<Vec<PlaceSubmission>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {revoked} = 1 AND {ticket_url} IS NOT NULL
            ORDER BY {updated_at} DESC, {id} DESC
        "#,
        projection = PlaceSubmission::projection(),
        table = schema::TABLE_NAME,
        revoked = Columns::Revoked.as_str(),
        ticket_url = Columns::TicketUrl.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(params![], PlaceSubmission::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_origin_counts_since(
    since: OffsetDateTime,
    conn: &Connection,
) -> Result<Vec<OriginSubmissionCounts>> {
    let sql = format!(
        r#"
            SELECT
                {origin} AS origin,
                COUNT(*) AS total,
                SUM(CASE WHEN {closed_at} IS NULL AND {revoked} = 0 THEN 1 ELSE 0 END) AS pending,
                SUM(CASE WHEN {revoked} = 1 THEN 1 ELSE 0 END) AS revoked
            FROM {table}
            WHERE {created_at} >= ?1
            GROUP BY {origin}
            ORDER BY {origin}
        "#,
        table = schema::TABLE_NAME,
        origin = Columns::Origin.as_str(),
        closed_at = Columns::ClosedAt.as_str(),
        revoked = Columns::Revoked.as_str(),
        created_at = Columns::CreatedAt.as_str(),
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![since.format(&Rfc3339)?], |row| {
        Ok(OriginSubmissionCounts {
            origin: row.get("origin")?,
            total: row.get("total")?,
            pending: row.get("pending")?,
            revoked: row.get("revoked")?,
        })
    })?;
    let mut res = vec![];
    for row in rows {
        res.push(row?);
    }
    Ok(res)
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<PlaceSubmission> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = PlaceSubmission::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![id], PlaceSubmission::mapper())
        .map_err(Into::into)
}

pub fn select_by_origin_and_external_id(
    origin: &str,
    external_id: &str,
    conn: &Connection,
) -> Result<Option<PlaceSubmission>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {origin} = ?1 AND {external_id} = ?2
        "#,
        projection = PlaceSubmission::projection(),
        table = schema::TABLE_NAME,
        origin = Columns::Origin.as_str(),
        external_id = Columns::ExternalId.as_str(),
    );
    conn.query_row(
        &sql,
        params![origin, external_id],
        PlaceSubmission::mapper(),
    )
    .optional()
    .map_err(Into::into)
}

pub fn set_fields(
    id: i64,
    lat: f64,
    lon: f64,
    category: &str,
    name: &str,
    extra_fields: JsonObject,
    conn: &Connection,
) -> Result<PlaceSubmission> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {lat} = :lat, {lon} = :lon, {category} = :category, {name} = :name, {extra_fields} = json(:extra_fields)
            WHERE {id} = :id
        "#,
        table = schema::TABLE_NAME,
        lat = Columns::Lat.as_str(),
        lon = Columns::Lon.as_str(),
        category = Columns::Category.as_str(),
        name = Columns::Name.as_str(),
        extra_fields = Columns::ExtraFields.as_str(),
        id = Columns::Id.as_str(),
    );
    let _rows = conn.execute(
        &sql,
        named_params! {
            ":id": id,
            ":lat": lat,
            ":lon": lon,
            ":category": category,
            ":name": name,
            ":extra_fields": Value::Object(extra_fields),
        },
    )?;
    select_by_id(id, conn)
}

pub fn set_revoked(id: i64, revoked: bool, conn: &Connection) -> Result<PlaceSubmission> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {revoked} = ?2
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        revoked = Columns::Revoked.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![id, revoked])?;
    select_by_id(id, conn)
}

pub fn set_ticket_url(id: i64, ticket_url: String, conn: &Connection) -> Result<PlaceSubmission> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {ticket_url} = ?2
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        ticket_url = Columns::TicketUrl.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(&sql, params![id, ticket_url])?;
    select_by_id(id, conn)
}

#[cfg(test)]
pub fn set_updated_at(
    id: i64,
    updated_at: time::OffsetDateTime,
    conn: &Connection,
) -> Result<PlaceSubmission> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {updated_at} = ?2
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.execute(
        &sql,
        params![
            id,
            updated_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap()
        ],
    )?;
    select_by_id(id, conn)
}

pub fn set_closed_at(
    id: i64,
    closed_at: Option<OffsetDateTime>,
    conn: &Connection,
) -> Result<PlaceSubmission> {
    match closed_at {
        Some(closed_at) => {
            let sql = format!(
                r#"
                    UPDATE {table}
                    SET {closed_at} = ?2
                    WHERE {id} = ?1
                "#,
                table = schema::TABLE_NAME,
                closed_at = Columns::ClosedAt.as_str(),
                id = Columns::Id.as_str(),
            );
            conn.execute(&sql, params![id, closed_at.format(&Rfc3339)?,])?;
        }
        None => {
            let sql = format!(
                r#"
                    UPDATE {table}
                    SET {closed_at} = NULL
                    WHERE {id} = ?1
                "#,
                table = schema::TABLE_NAME,
                closed_at = Columns::ClosedAt.as_str(),
                id = Columns::Id.as_str(),
            );
            conn.execute(&sql, params![id])?;
        }
    };
    select_by_id(id, conn)
}

#[cfg(test)]
mod test {
    use crate::db::main::place_submission::blocking_queries::InsertArgs;
    use crate::db::main::test::conn;
    use crate::Result;
    use geojson::JsonObject;
    use serde_json::Map;
    use time::macros::datetime;
    use time::OffsetDateTime;

    #[test]
    fn insert() -> Result<()> {
        let conn = conn();

        let origin = "acme";
        let external_id = "15";
        let lat = 1.23;
        let lon = 4.56;
        let category = "cafe";
        let name = "Satoshi Cafe";
        let extra_fields = Map::new();

        let args = InsertArgs {
            origin: origin.to_string(),
            external_id: external_id.to_string(),
            lat,
            lon,
            category: category.to_string(),
            name: name.to_string(),
            extra_fields: extra_fields.clone(),
        };
        let element = super::insert(&args, &conn)?;

        assert_eq!(extra_fields, element.extra_fields);

        let element = super::select_by_id(1, &conn)?;

        assert_eq!(origin, element.origin);
        assert_eq!(external_id, element.external_id);
        assert_eq!(lat, element.lat);
        assert_eq!(lon, element.lon);
        assert_eq!(category, element.category);
        assert_eq!(name, element.name);
        assert_eq!(extra_fields, element.extra_fields);

        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = conn();

        let origin = "acme";
        let external_id = "15";
        let lat = 1.23;
        let lon = 4.56;
        let category = "cafe";
        let name = "Satoshi Cafe";
        let extra_fields = Map::new();

        let args = InsertArgs {
            origin: origin.to_string(),
            external_id: external_id.to_string(),
            lat,
            lon,
            category: category.to_string(),
            name: name.to_string(),
            extra_fields: extra_fields.clone(),
        };
        let submission = super::insert(&args, &conn)?;

        assert_eq!(
            Some(submission),
            super::select_by_origin_and_external_id(origin.into(), external_id.into(), &conn)?
        );
        assert_eq!(
            None,
            super::select_by_origin_and_external_id(external_id.into(), origin.into(), &conn)?
        );

        Ok(())
    }

    #[test]
    fn set_fields() -> Result<()> {
        let conn = conn();

        let origin = "acme";
        let external_id = "15";
        let lat = 1.23;
        let lon = 4.56;
        let category = "cafe";
        let name = "Satoshi Cafe";
        let extra_fields = Map::new();

        let args = InsertArgs {
            origin: origin.to_string(),
            external_id: external_id.to_string(),
            lat,
            lon,
            category: category.to_string(),
            name: name.to_string(),
            extra_fields: extra_fields.clone(),
        };
        let submission = super::insert(&args, &conn)?;

        let new_lat = 1.11;
        let new_lon = 2.22;
        let new_category = "bar";
        let new_name = "FooBar";
        let mut new_extra_fields = JsonObject::new();
        new_extra_fields.insert("foo".into(), "bar".into());

        let submission = super::set_fields(
            submission.id,
            new_lat,
            new_lon,
            new_category,
            new_name,
            new_extra_fields.clone(),
            &conn,
        )?;

        assert_eq!(new_lat, submission.lat);
        assert_eq!(new_lon, submission.lon);
        assert_eq!(new_category, submission.category);
        assert_eq!(new_name, submission.name);
        assert_eq!(new_extra_fields, submission.extra_fields);

        Ok(())
    }

    #[test]
    fn set_revoked() -> Result<()> {
        let conn = conn();

        let args = InsertArgs {
            origin: "foo".to_string(),
            external_id: "bar".to_string(),
            lat: 1.11,
            lon: 2.22,
            category: "category".to_string(),
            name: "name".to_string(),
            extra_fields: JsonObject::new(),
        };
        let submission = super::insert(&args, &conn)?;

        assert_eq!(false, submission.revoked);

        let submission = super::set_revoked(submission.id, true, &conn)?;

        assert_eq!(true, submission.revoked);

        let submission = super::set_revoked(submission.id, false, &conn)?;

        assert_eq!(false, submission.revoked);

        Ok(())
    }

    #[test]
    fn set_updated_at() -> Result<()> {
        let conn = conn();

        let origin = "acme";
        let external_id = "15";
        let lat = 1.23;
        let lon = 4.56;
        let category = "cafe";
        let name = "Satoshi Cafe";
        let extra_fields = Map::new();

        let args = InsertArgs {
            origin: origin.to_string(),
            external_id: external_id.to_string(),
            lat,
            lon,
            category: category.to_string(),
            name: name.to_string(),
            extra_fields,
        };
        let submission = super::insert(&args, &conn)?;

        let updated_at = OffsetDateTime::now_utc();

        let submission = super::set_updated_at(submission.id, updated_at, &conn)?;

        assert_eq!(
            updated_at,
            super::select_by_id(submission.id, &conn)?.updated_at,
        );
        Ok(())
    }

    #[test]
    fn select_origin_counts_since_groups_by_origin() -> Result<()> {
        let conn = conn();

        let insert = |origin: &str, external_id: &str| -> Result<i64> {
            let args = InsertArgs {
                origin: origin.to_string(),
                external_id: external_id.to_string(),
                lat: 1.0,
                lon: 2.0,
                category: "cafe".to_string(),
                name: "Place".to_string(),
                extra_fields: Map::new(),
            };
            Ok(super::insert(&args, &conn)?.id)
        };

        let square_recent_1 = insert("square", "1")?;
        let square_recent_2 = insert("square", "2")?;
        let square_old = insert("square", "3")?;
        let coinos_recent = insert("coinos", "1")?;
        let coinos_closed = insert("coinos", "2")?;
        let coinos_revoked = insert("coinos", "3")?;
        let coinos_old = insert("coinos", "4")?;

        for id in [square_old, coinos_old] {
            conn.execute(
                "UPDATE place_submission SET created_at = '2020-01-01T00:00:00Z' WHERE id = ?1",
                rusqlite::params![id],
            )?;
        }

        super::set_closed_at(coinos_closed, Some(datetime!(2024-06-01 00:00 UTC)), &conn)?;
        super::set_revoked(coinos_revoked, true, &conn)?;

        let _ = square_recent_1;
        let _ = square_recent_2;
        let _ = coinos_recent;

        let counts = super::select_origin_counts_since(datetime!(2024-01-01 00:00 UTC), &conn)?;
        assert_eq!(
            vec![
                super::schema::OriginSubmissionCounts {
                    origin: "coinos".to_string(),
                    total: 3,
                    pending: 1,
                    revoked: 1,
                },
                super::schema::OriginSubmissionCounts {
                    origin: "square".to_string(),
                    total: 2,
                    pending: 2,
                    revoked: 0,
                },
            ],
            counts
        );

        let counts = super::select_origin_counts_since(datetime!(2019-01-01 00:00 UTC), &conn)?;
        assert_eq!(
            vec![
                super::schema::OriginSubmissionCounts {
                    origin: "coinos".to_string(),
                    total: 4,
                    pending: 2,
                    revoked: 1,
                },
                super::schema::OriginSubmissionCounts {
                    origin: "square".to_string(),
                    total: 3,
                    pending: 3,
                    revoked: 0,
                },
            ],
            counts
        );

        let counts = super::select_origin_counts_since(datetime!(2030-01-01 00:00 UTC), &conn)?;
        assert!(counts.is_empty());

        Ok(())
    }

    #[test]
    fn select_revoked_with_ticket_url() -> Result<()> {
        let conn = conn();

        let args = InsertArgs {
            origin: "foo".to_string(),
            external_id: "1".to_string(),
            lat: 1.0,
            lon: 2.0,
            category: "cafe".to_string(),
            name: "Place 1".to_string(),
            extra_fields: Map::new(),
        };
        let submission = super::insert(&args, &conn)?;

        let results = super::select_revoked_with_ticket_url(&conn)?;
        assert!(results.is_empty());

        super::set_revoked(submission.id, true, &conn)?;
        let results = super::select_revoked_with_ticket_url(&conn)?;
        assert!(results.is_empty());

        super::set_ticket_url(
            submission.id,
            "https://gitea.btcmap.org/api/v1/repos/teambtcmap/btcmap-data/issues/1".to_string(),
            &conn,
        )?;
        let results = super::select_revoked_with_ticket_url(&conn)?;
        assert_eq!(1, results.len());
        assert_eq!(submission.id, results[0].id);
        assert!(results[0].revoked);
        assert!(results[0].ticket_url.is_some());

        Ok(())
    }
}
