use super::schema::{self, Columns, PlaceSubmission};
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

pub fn insert(
    origin: &str,
    external_id: &str,
    lat: f64,
    lon: f64,
    category: &str,
    name: &str,
    extra_fields: &Map<String, Value>,
    conn: &Connection,
) -> Result<PlaceSubmission> {
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
            ":origin": origin,
            ":external_id": external_id,
            ":lat": lat,
            ":lon": lon,
            ":category": category,
            ":name": name,
            ":extra_fields": serde_json::to_string(extra_fields)?,

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

pub fn select_pending_by_bbox(
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
    conn: &Connection,
) -> Result<Vec<PlaceSubmission>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {lat} BETWEEN :min_lat AND :max_lat AND {lon} BETWEEN :min_lon AND :max_lon AND {deleted_at} IS NULL AND {closed_at} IS NULL AND {revoked} = 0
        "#,
        projection = PlaceSubmission::projection(),
        table = schema::TABLE_NAME,
        lat = Columns::Lat.as_str(),
        lon = Columns::Lon.as_str(),
        deleted_at = Columns::DeletedAt.as_str(),
        closed_at = Columns::ClosedAt.as_str(),
        revoked = Columns::Revoked.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(named_params! { ":min_lat": min_lat, ":max_lat": max_lat, ":min_lon": min_lon, ":max_lon": max_lon }, PlaceSubmission::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_search_query(
    search_query: impl Into<String>,
    include_deleted_and_closed: bool,
    conn: &Connection,
) -> Result<Vec<PlaceSubmission>> {
    let include_deleted_and_closed = if include_deleted_and_closed {
        ""
    } else {
        "AND deleted_at IS NULL AND closed_at IS NULL"
    };
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE LOWER({name}) LIKE '%' || UPPER(?1) || '%' {include_deleted_and_closed}
            ORDER BY {updated_at}, {id}
        "#,
        projection = PlaceSubmission::projection(),
        table = schema::TABLE_NAME,
        name = Columns::Name.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(params![search_query.into()], PlaceSubmission::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_origin(origin: &str, conn: &Connection) -> Result<Vec<PlaceSubmission>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {origin} = ?1 AND deleted_at IS NULL AND closed_at IS NULL
        "#,
        projection = PlaceSubmission::projection(),
        table = schema::TABLE_NAME,
        origin = Columns::Origin.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(params![origin], PlaceSubmission::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_updated_since(
    updated_since: OffsetDateTime,
    limit: Option<i64>,
    include_deleted_and_closed: bool,
    conn: &Connection,
) -> Result<Vec<PlaceSubmission>> {
    let sql = if include_deleted_and_closed {
        format!(
            r#"
                SELECT {projection}
                FROM {table}
                WHERE julianday({updated_at}) > julianday(:updated_since)
                ORDER BY {updated_at}, {id}
                LIMIT :limit
            "#,
            projection = PlaceSubmission::projection(),
            table = schema::TABLE_NAME,
            updated_at = Columns::UpdatedAt.as_str(),
            id = Columns::Id.as_str(),
        )
    } else {
        format!(
            r#"
                SELECT {projection}
                FROM {table}
                WHERE {deleted_at} IS NULL AND {closed_at} IS NULL AND julianday({updated_at}) > julianday(:updated_since)
                ORDER BY {updated_at}, {id}
                LIMIT :limit
            "#,
            projection = PlaceSubmission::projection(),
            table = schema::TABLE_NAME,
            deleted_at = Columns::DeletedAt.as_str(),
            closed_at = Columns::ClosedAt.as_str(),
            updated_at = Columns::UpdatedAt.as_str(),
            id = Columns::Id.as_str(),
        )
    };
    Ok(conn
        .prepare(&sql)?
        .query_map(
            named_params! {
                ":updated_since": updated_since.format(&Rfc3339)?,
                ":limit": limit.unwrap_or(i64::MAX)
            },
            PlaceSubmission::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()?)
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
    use crate::db::test::conn;
    use crate::error::Error;
    use crate::Result;
    use geojson::JsonObject;
    use serde_json::Map;
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

        let element = super::insert(
            origin,
            external_id,
            lat,
            lon,
            category,
            name,
            &extra_fields,
            &conn,
        )?;

        assert_eq!(origin, element.origin);
        assert_eq!(external_id, element.external_id);
        assert_eq!(lat, element.lat);
        assert_eq!(lon, element.lon);
        assert_eq!(category, element.category);
        assert_eq!(name, element.name);
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

        let submission = super::insert(
            origin,
            external_id,
            lat,
            lon,
            category,
            name,
            &extra_fields,
            &conn,
        )?;

        assert_eq!(submission, super::select_by_id(submission.id, &conn)?);

        Ok(())
    }

    #[test]
    fn select_by_id_not_found() {
        assert!(matches!(
            super::select_by_id(1, &conn()),
            Err(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows)),
        ));
    }

    #[test]
    fn select_by_origin_and_external_id() -> Result<()> {
        let conn = conn();

        let origin = "acme";
        let external_id = "15";
        let lat = 1.23;
        let lon = 4.56;
        let category = "cafe";
        let name = "Satoshi Cafe";
        let extra_fields = Map::new();

        let submission = super::insert(
            origin,
            external_id,
            lat,
            lon,
            category,
            name,
            &extra_fields,
            &conn,
        )?;

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
    fn select_pending_by_bbox() -> Result<()> {
        let conn = conn();

        let origin = "acme";
        let external_id = "15";
        let lat = 0.0;
        let lon = 0.0;
        let category = "cafe";
        let name = "Satoshi Cafe";
        let extra_fields = Map::new();

        let submission = super::insert(
            origin,
            external_id,
            lat,
            lon,
            category,
            name,
            &extra_fields,
            &conn,
        )?;

        assert_eq!(
            vec![submission.clone()],
            super::select_pending_by_bbox(-1.0, 1.0, -1.0, 1.0, &conn)?
        );

        super::set_revoked(submission.id, true, &conn)?;

        assert!(super::select_pending_by_bbox(-1.0, 1.0, -1.0, 1.0, &conn)?.is_empty());

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

        let submission = super::insert(
            origin,
            external_id,
            lat,
            lon,
            category,
            name,
            &extra_fields,
            &conn,
        )?;

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

        let submission = super::insert(
            "foo",
            "bar",
            1.11,
            2.22,
            "category",
            "name",
            &JsonObject::new(),
            &conn,
        )?;

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

        let submission = super::insert(
            origin,
            external_id,
            lat,
            lon,
            category,
            name,
            &extra_fields,
            &conn,
        )?;

        let updated_at = OffsetDateTime::now_utc();

        let submission = super::set_updated_at(submission.id, updated_at, &conn)?;

        assert_eq!(
            updated_at,
            super::select_by_id(submission.id, &conn)?.updated_at,
        );
        Ok(())
    }
}
