use super::schema::{self, Columns, PlaceSubmission};
use crate::Result;
use geojson::JsonObject;
use rusqlite::{named_params, params, Connection, OptionalExtension};
use serde_json::{Map, Value};

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

// pub fn set_closed_at(
//     id: i64,
//     closed_at: Option<OffsetDateTime>,
//     conn: &Connection,
// ) -> Result<PlaceSubmission> {
//     match closed_at {
//         Some(closed_at) => {
//             let sql = format!(
//                 r#"
//                     UPDATE {table}
//                     SET {closed_at} = ?2
//                     WHERE {id} = ?1
//                 "#,
//                 table = schema::TABLE_NAME,
//                 closed_at = Columns::ClosedAt.as_str(),
//                 id = Columns::Id.as_str(),
//             );
//             conn.execute(&sql, params![id, closed_at.format(&Rfc3339)?,])?;
//         }
//         None => {
//             let sql = format!(
//                 r#"
//                     UPDATE {table}
//                     SET {closed_at} = NULL
//                     WHERE {id} = ?1
//                 "#,
//                 table = schema::TABLE_NAME,
//                 closed_at = Columns::ClosedAt.as_str(),
//                 id = Columns::Id.as_str(),
//             );
//             conn.execute(&sql, params![id])?;
//         }
//     };
//     select_by_id(id, conn)
// }

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
