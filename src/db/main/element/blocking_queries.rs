use super::schema::{self, Columns, Element};
use crate::db::main::area_element::schema::{self as area_element_schema};
use crate::service::search::{escape_like, split_words};
use crate::{service::overpass::OverpassElement, Result};
use rusqlite::types::Value as SqlValue;
use rusqlite::{named_params, params, params_from_iter, Connection};
use serde_json::{Map, Value};
use time::{format_description::well_known::Rfc3339, Date, OffsetDateTime};

pub fn insert(overpass_data: &OverpassElement, conn: &Connection) -> Result<Element> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({overpass_data}) 
            VALUES (json(?1))
            RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        overpass_data = Columns::OverpassData.as_ref(),
        projection = Element::projection(),
    );
    conn.query_row(
        &sql,
        params![serde_json::to_string(overpass_data)?],
        Element::mapper(),
    )
    .map_err(Into::into)
}

pub fn select_updated_since(
    updated_since: OffsetDateTime,
    limit: Option<i64>,
    include_deleted: bool,
    conn: &Connection,
) -> Result<Vec<Element>> {
    let sql = if include_deleted {
        format!(
            r#"
                SELECT {projection}
                FROM {table}
                WHERE julianday({updated_at}) > julianday(:updated_since)
                ORDER BY {updated_at}, {id}
                LIMIT :limit
            "#,
            projection = Element::projection(),
            table = schema::TABLE_NAME,
            updated_at = Columns::UpdatedAt.as_ref(),
            id = Columns::Id.as_ref(),
        )
    } else {
        format!(
            r#"
                SELECT {projection}
                FROM {table}
                WHERE {deleted_at} IS NULL AND julianday({updated_at}) > julianday(:updated_since)
                ORDER BY {updated_at}, {id}
                LIMIT :limit
            "#,
            projection = Element::projection(),
            table = schema::TABLE_NAME,
            deleted_at = Columns::DeletedAt.as_ref(),
            updated_at = Columns::UpdatedAt.as_ref(),
            id = Columns::Id.as_ref(),
        )
    };
    Ok(conn
        .prepare(&sql)?
        .query_map(
            named_params! {
                ":updated_since": updated_since.format(&Rfc3339)?,
                ":limit": limit.unwrap_or(i64::MAX)
            },
            Element::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn select_by_search_query(
    search_query: impl Into<String>,
    include_deleted: bool,
    conn: &Connection,
) -> Result<Vec<Element>> {
    let include_deleted = if include_deleted {
        ""
    } else {
        "AND deleted_at IS NULL"
    };
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE LOWER(json_extract({overpass_data}, '$.tags.name')) LIKE '%' || UPPER(?1) || '%' {include_deleted}
            ORDER BY {updated_at}, {id}
        "#,
        projection = Element::projection(),
        table = schema::TABLE_NAME,
        overpass_data = Columns::OverpassData.as_ref(),
        updated_at = Columns::UpdatedAt.as_ref(),
        id = Columns::Id.as_ref(),
    );
    conn.prepare(&sql)?
        .query_map(params![search_query.into()], Element::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_bbox(
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
    conn: &Connection,
) -> Result<Vec<Element>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {lat} BETWEEN :min_lat AND :max_lat AND {lon} BETWEEN :min_lon AND :max_lon AND {deleted_at} IS NULL
        "#,
        projection = Element::projection(),
        table = schema::TABLE_NAME,
        lat = Columns::Lat.as_ref(),
        lon = Columns::Lon.as_ref(),
        deleted_at = Columns::DeletedAt.as_ref(),
    );
    conn.prepare(&sql)?
        .query_map(named_params! { ":min_lat": min_lat, ":max_lat": max_lat, ":min_lon": min_lon, ":max_lon": max_lon }, Element::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_osm_tag_value(
    tag_name: &str,
    tag_value: &str,
    conn: &Connection,
) -> Result<Vec<Element>> {
    // sanitizing is a MUST!
    let tag_name: String = tag_name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == ':')
        .collect();

    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE json_extract({overpass_data}, '$.tags.{tag_name}') = ? AND deleted_at IS NULL
        "#,
        projection = Element::projection(),
        table = schema::TABLE_NAME,
        overpass_data = Columns::OverpassData.as_ref(),
    );
    conn.prepare(&sql)?
        .query_map(params![tag_value], Element::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_with_opening_hours_without_humanization(
    limit: i64,
    conn: &Connection,
) -> Result<Vec<Element>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE json_extract({overpass_data}, '$.tags.opening_hours') IS NOT NULL
            AND json_extract({tags}, '$.opening_hours:en:human_readable') IS NULL
            AND deleted_at IS NULL
            ORDER BY RANDOM()
            LIMIT ?
        "#,
        projection = Element::projection(),
        table = schema::TABLE_NAME,
        overpass_data = Columns::OverpassData.as_ref(),
        tags = Columns::Tags.as_ref(),
    );
    conn.prepare(&sql)?
        .query_map(params![limit], Element::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_with_opening_hours_without_humanization_by_area(
    area_id: i64,
    limit: i64,
    conn: &Connection,
) -> Result<Vec<Element>> {
    let table = schema::TABLE_NAME;
    let sql = format!(
        r#"
            SELECT {table}.{id}, {table}.{overpass_data}, {table}.{tags}, {table}.{lat}, {table}.{lon}, {table}.{created_at}, {table}.{updated_at}, {table}.{deleted_at}
            FROM {table}
            INNER JOIN {area_element_table} ae ON ae.element_id = {table}.id AND ae.area_id = ?1 AND ae.deleted_at IS NULL
            WHERE json_extract({table}.{overpass_data}, '$.tags.opening_hours') IS NOT NULL
            AND json_extract({table}.{tags}, '$.opening_hours:en:human_readable') IS NULL
            AND {table}.{deleted_at} IS NULL
            ORDER BY RANDOM()
            LIMIT ?2
        "#,
        table = table,
        id = Columns::Id.as_ref(),
        overpass_data = Columns::OverpassData.as_ref(),
        tags = Columns::Tags.as_ref(),
        lat = Columns::Lat.as_ref(),
        lon = Columns::Lon.as_ref(),
        created_at = Columns::CreatedAt.as_ref(),
        updated_at = Columns::UpdatedAt.as_ref(),
        deleted_at = Columns::DeletedAt.as_ref(),
        area_element_table = area_element_schema::TABLE_NAME,
    );
    conn.prepare(&sql)?
        .query_map(params![area_id, limit], Element::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_id_or_osm_id(id: impl Into<String>, conn: &Connection) -> Result<Element> {
    let id: String = id.into();
    let id = id.as_str();
    match id.parse::<i64>() {
        Ok(id) => select_by_id(id, conn),
        Err(_) => {
            let parts: Vec<_> = id.split(':').collect();
            let osm_type = parts[0];
            let osm_id = parts
                .get(1)
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName(format!("Invalid id format: {}", id))
                })?;
            select_by_osm_type_and_id(osm_type, osm_id, conn)
        }
    }
}

pub fn select_by_id(id: i64, conn: &Connection) -> Result<Element> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} = ?1
        "#,
        projection = Element::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_ref(),
    );
    conn.query_row(&sql, params![id], Element::mapper())
        .map_err(Into::into)
}

pub fn select_by_ids(ids: &[i64], conn: &Connection) -> Result<Vec<Element>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE {id} IN ({placeholders})
            AND deleted_at IS NULL
        "#,
        projection = Element::projection(),
        table = schema::TABLE_NAME,
        id = Columns::Id.as_ref(),
        placeholders = placeholders.join(", "),
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::ToSql> =
        ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
    let mut rows = stmt.query(params.as_slice())?;
    let mut elements = Vec::new();
    while let Some(row) = rows.next()? {
        elements.push(Element::mapper()(row)?);
    }
    Ok(elements)
}

pub fn select_by_osm_type_and_id(
    osm_type: &str,
    osm_id: i64,
    conn: &Connection,
) -> Result<Element> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE json_extract({overpass_data}, '$.type') = ?1
            AND json_extract({overpass_data}, '$.id') = ?2
        "#,
        projection = Element::projection(),
        table = schema::TABLE_NAME,
        overpass_data = Columns::OverpassData.as_ref(),
    );
    Ok(conn.query_row(&sql, params![osm_type, osm_id], Element::mapper())?)
}

pub fn select_merchants_count(conn: &Connection, verified_since: Option<Date>) -> Result<i64> {
    let mut sql = format!(
        r#"
            SELECT COUNT(*)
            FROM {table}
            WHERE {deleted_at} IS NULL AND coalesce(json_extract({overpass_data}, '$.tags.amenity'), '') != 'atm' AND coalesce(json_extract({overpass_data}, '$.tags.amenity'), '') != 'bureau_de_change'
        "#,
        table = schema::TABLE_NAME,
        deleted_at = Columns::DeletedAt.as_ref(),
        overpass_data = Columns::OverpassData.as_ref(),
    );
    if let Some(verified_since) = verified_since {
        let verified_since = verified_since.to_string();
        sql.push_str(&format!(" AND (coalesce(json_extract({overpass_data}, '$.tags.survey:date'), '2000-01-01') > '{verified_since}' OR coalesce(json_extract({overpass_data}, '$.tags.check_date'), '2000-01-01') > '{verified_since}' OR coalesce(json_extract({overpass_data}, '$.tags.check_date:currency:XBT'), '2000-01-01') > '{verified_since}')", overpass_data = Columns::OverpassData.as_ref()));
    }
    Ok(conn.query_row(&sql, [], |row| row.get(0))?)
}

pub fn select_exchanges_count(conn: &Connection, verified_since: Option<Date>) -> Result<i64> {
    let mut sql = format!(
        r#"
            SELECT COUNT(*)
            FROM {table}
            WHERE {deleted_at} IS NULL AND (coalesce(json_extract({overpass_data}, '$.tags.amenity'), '') = 'atm' OR coalesce(json_extract({overpass_data}, '$.tags.amenity'), '') = 'bureau_de_change')
        "#,
        table = schema::TABLE_NAME,
        deleted_at = Columns::DeletedAt.as_ref(),
        overpass_data = Columns::OverpassData.as_ref(),
    );
    if let Some(verified_since) = verified_since {
        let verified_since = verified_since.to_string();
        sql.push_str(&format!(" AND (json_extract({overpass_data}, '$.tags.survey:date') > '{verified_since}' or json_extract({overpass_data}, '$.tags.check_date') > '{verified_since}' or json_extract({overpass_data}, '$.tags.check_date:currency:XBT') > '{verified_since}')", overpass_data = Columns::OverpassData.as_ref()));
    }
    Ok(conn.query_row(&sql, [], |row| row.get(0))?)
}

pub fn set_overpass_data(
    id: i64,
    overpass_data: &OverpassElement,
    conn: &Connection,
) -> Result<Element> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {overpass_data} = json(?2)
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        overpass_data = Columns::OverpassData.as_ref(),
        id = Columns::Id.as_ref(),
    );
    conn.execute(&sql, params![id, serde_json::to_string(overpass_data)?,])?;
    select_by_id(id, conn)
}

pub fn patch_tags(id: i64, tags: &Map<String, Value>, conn: &Connection) -> Result<Element> {
    let sql = format!(
        r#"
            UPDATE {table} SET {tags} = json_patch({tags}, ?2) WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        tags = Columns::Tags.as_ref(),
        id = Columns::Id.as_ref(),
    );
    conn.execute(&sql, params![id, &serde_json::to_string(tags)?,])?;
    select_by_id(id, conn)
}

pub fn set_tag(id: i64, name: &str, value: &Value, conn: &Connection) -> Result<Element> {
    let mut patch_set = Map::new();
    patch_set.insert(name.into(), value.clone());
    patch_tags(id, &patch_set, conn)
}

pub fn remove_tag(element_id: i64, tag_name: &str, conn: &Connection) -> Result<Element> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {tags} = json_remove({tags}, ?2)
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        tags = Columns::Tags.as_ref(),
        id = Columns::Id.as_ref(),
    );
    conn.execute(&sql, params![element_id, format!("$.{tag_name}"),])?;
    select_by_id(element_id, conn)
}

pub fn set_lat_lon(id: i64, lat: f64, lon: f64, conn: &Connection) -> Result<Element> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {lat} = ?2, {lon} = ?3
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        lat = Columns::Lat.as_ref(),
        lon = Columns::Lon.as_ref(),
        id = Columns::Id.as_ref(),
    );
    conn.execute(&sql, params![id, lat, lon])?;
    select_by_id(id, conn)
}

#[cfg(test)]
pub fn set_updated_at(id: i64, updated_at: OffsetDateTime, conn: &Connection) -> Result<Element> {
    let sql = format!(
        r#"
            UPDATE {table}
            SET {updated_at} = ?2
            WHERE {id} = ?1
        "#,
        table = schema::TABLE_NAME,
        updated_at = Columns::UpdatedAt.as_ref(),
        id = Columns::Id.as_ref(),
    );
    conn.execute(&sql, params![id, updated_at.format(&Rfc3339).unwrap()])?;
    select_by_id(id, conn)
}

pub fn set_deleted_at(
    id: i64,
    deleted_at: Option<OffsetDateTime>,
    conn: &Connection,
) -> Result<Element> {
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
            conn.execute(&sql, params![id, deleted_at.format(&Rfc3339)?,])?;
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

#[derive(Debug, PartialEq)]
pub struct RankedElement {
    pub element: Element,
    pub rank: i64,
}

/// Shared predicate for both the select and the count. `first_word_param` is the
/// 1-based index of the first `?N` placeholder that holds a word pattern, so the
/// two callers can bind different numbers of leading parameters.
///
/// `json_type(...) = 'object'` guards `json_each` against a NULL argument for
/// elements with no tags: SQLite does not promise `WHERE`-clause evaluation
/// order, so the `name IS NOT NULL` check cannot be relied on to shield it.
fn tag_value_predicate(word_count: usize, first_word_param: usize) -> String {
    let mut words = String::new();
    for i in 0..word_count {
        words.push_str(&format!(
            r#"
            AND EXISTS (
                SELECT 1 FROM json_each(json_extract({table}.{overpass_data}, '$.tags')) t
                WHERE t.type = 'text' AND t.value LIKE ?{param} ESCAPE '\'
            )"#,
            table = schema::TABLE_NAME,
            overpass_data = Columns::OverpassData.as_ref(),
            param = first_word_param + i,
        ));
    }
    format!(
        r#"{deleted_at} IS NULL
            AND {lat} IS NOT NULL
            AND {lon} IS NOT NULL
            AND json_type({overpass_data}, '$.tags') = 'object'
            AND json_extract({overpass_data}, '$.tags.name') IS NOT NULL{words}"#,
        deleted_at = Columns::DeletedAt.as_ref(),
        lat = Columns::Lat.as_ref(),
        lon = Columns::Lon.as_ref(),
        overpass_data = Columns::OverpassData.as_ref(),
    )
}

fn word_patterns(words: &[String]) -> impl Iterator<Item = SqlValue> + '_ {
    words
        .iter()
        .map(|word| SqlValue::Text(format!("%{}%", escape_like(word))))
}

/// Matches `query` against every OSM tag **value** (never a key), requiring every
/// word to match some value. Ranks exact name hits above prefix, above infix,
/// above a hit on any other tag. `location` breaks rank ties by proximity.
pub fn select_by_tag_value_search(
    query: &str,
    location: Option<(f64, f64)>,
    row_limit: i64,
    conn: &Connection,
) -> Result<Vec<RankedElement>> {
    let words = split_words(query);
    // Rank and order on the same name the response shows — `name(Some("en"))`,
    // i.e. name:en when present, else the raw name. Ranking on the raw name while
    // displaying name:en both mis-ranks localized matches and, because the DB
    // order must equal the in-memory merge order for the reslice to be correct,
    // could drop a row at a page boundary. The `IS NOT NULL` gate in the
    // predicate stays on the raw name, so this COALESCE is never null.
    let name = format!(
        r#"COALESCE(NULLIF(json_extract({od}, '$.tags."name:en"'), ''), json_extract({od}, '$.tags.name'))"#,
        od = Columns::OverpassData.as_ref(),
    );
    let sql = format!(
        r#"
            SELECT {projection},
              CASE
                WHEN {name} = ?1 COLLATE NOCASE THEN 0
                WHEN {name} LIKE ?2 ESCAPE '\' THEN 1
                WHEN {name} LIKE ?3 ESCAPE '\' THEN 2
                ELSE 3
              END AS search_rank
            FROM {table}
            WHERE {predicate}
            ORDER BY search_rank,
                     CASE WHEN ?6 = 1
                          THEN ({lat} - ?4) * ({lat} - ?4) + ({lon} - ?5) * ({lon} - ?5)
                          ELSE 0 END,
                     LENGTH({name}),
                     {name},
                     {id}
            LIMIT ?7
        "#,
        projection = Element::projection(),
        table = schema::TABLE_NAME,
        predicate = tag_value_predicate(words.len(), 8),
        lat = Columns::Lat.as_ref(),
        lon = Columns::Lon.as_ref(),
        id = Columns::Id.as_ref(),
    );

    let escaped = escape_like(query);
    let (lat, lon, has_location) = match location {
        Some((lat, lon)) => (lat, lon, 1),
        None => (0.0, 0.0, 0),
    };
    let mut values = vec![
        SqlValue::Text(query.to_string()),
        SqlValue::Text(format!("{escaped}%")),
        SqlValue::Text(format!("%{escaped}%")),
        SqlValue::Real(lat),
        SqlValue::Real(lon),
        SqlValue::Integer(has_location),
        SqlValue::Integer(row_limit),
    ];
    values.extend(word_patterns(&words));

    conn.prepare(&sql)?
        .query_map(params_from_iter(values), |row| {
            Ok(RankedElement {
                element: Element::mapper()(row)?,
                rank: row.get("search_rank")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn count_by_tag_value_search(query: &str, conn: &Connection) -> Result<i64> {
    let words = split_words(query);
    let sql = format!(
        "SELECT COUNT(*) FROM {table} WHERE {predicate}",
        table = schema::TABLE_NAME,
        predicate = tag_value_predicate(words.len(), 1),
    );
    let values: Vec<SqlValue> = word_patterns(&words).collect();
    conn.query_row(&sql, params_from_iter(values), |row| row.get(0))
        .map_err(Into::into)
}

#[cfg(test)]
mod test {
    use super::schema::Element;
    use crate::db::main::test::conn;
    use crate::service::overpass::OverpassElement;
    use crate::Error;
    use crate::Result;
    use serde_json::{json, Map};
    use time::macros::datetime;
    use time::OffsetDateTime;

    #[test]
    fn insert() -> Result<()> {
        let conn = conn();
        let overpass_data = OverpassElement::mock(1);
        let element = super::insert(&overpass_data, &conn)?;
        assert_eq!(overpass_data, element.overpass_data);
        let element = super::select_by_id(1, &conn)?;
        assert_eq!(overpass_data, element.overpass_data);
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = conn();
        let element_1 = super::insert(&OverpassElement::mock(1), &conn)?;
        let _element_1 =
            super::set_updated_at(element_1.id, datetime!(2023-10-01 00:00 UTC), &conn)?;
        let element_2 = super::insert(&OverpassElement::mock(2), &conn)?;
        let element_2 =
            super::set_updated_at(element_2.id, datetime!(2023-10-02 00:00 UTC), &conn)?;
        assert_eq!(
            vec![element_2.clone()],
            super::select_updated_since(datetime!(2023-10-01 00:00 UTC), None, false, &conn)?
        );
        super::set_deleted_at(element_2.id, Some(OffsetDateTime::now_utc()), &conn)?;
        assert_eq!(
            0,
            super::select_updated_since(datetime!(2023-10-01 00:00 UTC), None, false, &conn)?.len()
        );
        Ok(())
    }

    #[test]
    fn select_by_search_query() -> Result<()> {
        let conn = conn();

        // Insert test data with different name patterns
        let element1 = super::insert(
            &OverpassElement::mock_with_tag(1, "name", "Coffee Shop Downtown"),
            &conn,
        )?;
        let element2 = super::insert(
            &OverpassElement::mock_with_tag(2, "name", "Central Park"),
            &conn,
        )?;
        let element3 = super::insert(
            &OverpassElement::mock_with_tag(3, "name", "Downtown Mall"),
            &conn,
        )?;
        // Element without a name tag
        let _element4 = super::insert(&OverpassElement::mock(4), &conn)?;

        // Test case-insensitive matching
        assert_eq!(
            vec![element1.clone(), element3.clone()],
            super::select_by_search_query("downtown", false, &conn)?
        );

        // Test partial matching
        assert_eq!(
            vec![element2.clone()],
            super::select_by_search_query("park", false, &conn)?
        );

        // Test no matches
        assert_eq!(
            0,
            super::select_by_search_query("nonexistent", false, &conn)?.len()
        );

        // Test empty query (should match all named elements)
        let results = super::select_by_search_query("", false, &conn)?;
        assert_eq!(3, results.len());
        assert!(results.contains(&element1));
        assert!(results.contains(&element2));
        assert!(results.contains(&element3));

        Ok(())
    }

    #[test]
    fn select_by_search_query_special_chars() -> Result<()> {
        let conn = conn();

        let element = super::insert(
            &OverpassElement::mock_with_tag(1, "name", "Café 'Le Paris'"),
            &conn,
        )?;

        // Test with special characters
        assert_eq!(
            vec![element.clone()],
            super::select_by_search_query("café", false, &conn)?
        );

        assert_eq!(
            vec![element.clone()],
            super::select_by_search_query("paris", false, &conn)?
        );

        // Test with single quote
        assert_eq!(
            vec![element],
            super::select_by_search_query("le paris", false, &conn)?
        );

        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = conn();
        let element = super::insert(&OverpassElement::mock(1), &conn)?;
        assert_eq!(element, super::select_by_id(element.id, &conn)?);
        Ok(())
    }

    #[test]
    fn select_by_id_found() -> Result<()> {
        let conn = conn();

        let test_id = 1;
        let test_overpass_id = 2;

        let item = super::insert(&OverpassElement::mock(test_overpass_id), &conn)?;
        let item = super::select_by_id(item.id, &conn)?;

        assert_eq!(item.id, test_id);
        assert_eq!(item.overpass_data.id, test_overpass_id);

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
    fn select_by_osm_type_and_id() -> Result<()> {
        let conn = conn();
        let element = super::insert(&OverpassElement::mock(1), &conn)?;
        assert_eq!(
            element,
            super::select_by_osm_type_and_id(
                &element.overpass_data.r#type,
                element.overpass_data.id,
                &conn,
            )?
        );
        Ok(())
    }

    #[test]
    fn set_overpass_data() -> Result<()> {
        let conn = conn();
        let orig_data = OverpassElement::mock(1);
        let override_data = OverpassElement::mock(2);
        let element = super::insert(&orig_data, &conn)?;
        let element = super::set_overpass_data(element.id, &override_data, &conn)?;
        assert_eq!(override_data, element.overpass_data);
        Ok(())
    }

    #[test]
    fn patch_tags() -> Result<()> {
        let conn = conn();
        let tag_1_name = "tag_1_name";
        let tag_1_value_1 = json!("tag_1_value_1");
        let tag_1_value_2 = json!("tag_1_value_2");
        let tag_2_name = "tag_2_name";
        let tag_2_value = json!("tag_2_value");
        let element = super::insert(&OverpassElement::mock(1), &conn)?;
        let mut tags = Map::new();
        tags.insert(tag_1_name.into(), tag_1_value_1.clone());
        let element = super::patch_tags(element.id, &tags, &conn)?;
        assert_eq!(&tag_1_value_1, element.tag(tag_1_name));
        tags.insert(tag_1_name.into(), tag_1_value_2.clone());
        let element = super::patch_tags(element.id, &tags, &conn)?;
        assert_eq!(&tag_1_value_2, element.tag(tag_1_name));
        tags.clear();
        tags.insert(tag_2_name.into(), tag_2_value.clone());
        let element = super::patch_tags(element.id, &tags, &conn)?;
        assert!(element.tags.contains_key(tag_1_name));
        assert_eq!(&tag_2_value, element.tag(tag_2_name));
        Ok(())
    }

    #[test]
    fn set_tag() -> Result<()> {
        let conn = conn();
        let tag_name = "foo";
        let tag_value = json!("bar");
        let element = super::insert(&OverpassElement::mock(1), &conn)?;
        let element = super::set_tag(element.id, tag_name, &tag_value, &conn)?;
        assert_eq!(tag_value, element.tags[tag_name]);
        Ok(())
    }

    #[test]
    fn remove_tag() -> Result<()> {
        let conn = conn();
        let tag_name = "foo";
        let element = super::insert(&OverpassElement::mock(1), &conn)?;
        let element = super::set_tag(element.id, tag_name, &"bar".into(), &conn)?;
        let element = super::remove_tag(element.id, tag_name, &conn)?;
        assert!(!element.tags.contains_key(tag_name));
        Ok(())
    }

    #[test]
    fn set_lat_lon() -> Result<()> {
        let conn = conn();
        let lat = 1.23;
        let lon = 4.56;
        let element = super::insert(&OverpassElement::mock(1), &conn)?;
        let element = super::set_lat_lon(element.id, lat, lon, &conn)?;
        assert_eq!(Some(lat), super::select_by_id(element.id, &conn)?.lat);
        assert_eq!(Some(lon), super::select_by_id(element.id, &conn)?.lon);
        Ok(())
    }

    #[test]
    fn set_updated_at() -> Result<()> {
        let conn = conn();
        let updated_at = OffsetDateTime::now_utc();
        let element = super::insert(&OverpassElement::mock(1), &conn)?;
        let element = super::set_updated_at(element.id, updated_at, &conn)?;
        assert_eq!(
            updated_at,
            super::select_by_id(element.id, &conn)?.updated_at
        );
        Ok(())
    }

    #[test]
    fn set_deleted_at() -> Result<()> {
        let conn = conn();
        let deleted_at = OffsetDateTime::now_utc();
        let element = super::insert(&OverpassElement::mock(1), &conn)?;
        let element = super::set_deleted_at(element.id, Some(deleted_at), &conn)?;
        assert_eq!(
            deleted_at,
            super::select_by_id(element.id, &conn)?.deleted_at.unwrap()
        );
        Ok(())
    }

    #[test]
    fn select_by_id_or_osm_id_by_id() -> Result<()> {
        let conn = conn();
        let element = super::insert(&OverpassElement::mock(42), &conn)?;

        let found = super::select_by_id_or_osm_id(element.id.to_string(), &conn)?;
        assert_eq!(found.id, element.id);

        Ok(())
    }

    #[test]
    fn select_by_id_or_osm_id_by_osm_id() -> Result<()> {
        let conn = conn();
        let element = super::insert(&OverpassElement::mock(99), &conn)?;

        let osm_id = format!("node:{}", element.overpass_data.id);
        let found = super::select_by_id_or_osm_id(osm_id, &conn)?;
        assert_eq!(found.id, element.id);

        Ok(())
    }

    #[test]
    fn select_by_id_or_osm_id_invalid_format() -> Result<()> {
        let conn = conn();

        let result = super::select_by_id_or_osm_id("invalid_no_colon", &conn);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn select_by_ids_empty() {
        let conn = conn();
        let result = super::select_by_ids(&[], &conn);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn select_by_ids_not_found() {
        let conn = conn();
        let result = super::select_by_ids(&[999, 998], &conn);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn select_by_ids_found() {
        use crate::service::overpass::OverpassElement;
        let conn = conn();
        let element1 = super::insert(&OverpassElement::mock(1), &conn).unwrap();
        let element2 = super::insert(&OverpassElement::mock(2), &conn).unwrap();
        let result = super::select_by_ids(&[element1.id, element2.id], &conn).unwrap();
        assert_eq!(2, result.len());
    }

    fn insert_place(
        id: i64,
        tags: &[(&str, &str)],
        lat: f64,
        lon: f64,
        conn: &rusqlite::Connection,
    ) -> Element {
        let element = super::insert(&OverpassElement::mock_with_tags(id, tags), conn).unwrap();
        super::set_lat_lon(element.id, lat, lon, conn).unwrap()
    }

    fn search(query: &str, conn: &rusqlite::Connection) -> Vec<Element> {
        super::select_by_tag_value_search(query, None, 100, conn)
            .unwrap()
            .into_iter()
            .map(|it| it.element)
            .collect()
    }

    #[test]
    fn tag_value_search_matches_address_city() -> Result<()> {
        let conn = conn();
        let hit = insert_place(
            1,
            &[("name", "Kaffeeklatsch"), ("addr:city", "Hamburg")],
            53.5,
            9.9,
            &conn,
        );
        insert_place(
            2,
            &[("name", "Elsewhere"), ("addr:city", "Berlin")],
            52.5,
            13.4,
            &conn,
        );
        assert_eq!(vec![hit], search("hamburg", &conn));
        Ok(())
    }

    #[test]
    fn tag_value_search_matches_localized_name() -> Result<()> {
        let conn = conn();
        let hit = insert_place(
            1,
            &[("name", "Kaffeeklatsch"), ("name:ja", "カフェ")],
            53.5,
            9.9,
            &conn,
        );
        assert_eq!(vec![hit], search("カフェ", &conn));
        Ok(())
    }

    #[test]
    fn tag_value_search_never_matches_tag_keys() -> Result<()> {
        let conn = conn();
        insert_place(
            1,
            &[("name", "Kaffeeklatsch"), ("addr:city", "Hamburg")],
            53.5,
            9.9,
            &conn,
        );
        assert!(search("addr", &conn).is_empty());
        Ok(())
    }

    #[test]
    fn tag_value_search_skips_unnamed_elements() -> Result<()> {
        let conn = conn();
        insert_place(1, &[("addr:city", "Hamburg")], 53.5, 9.9, &conn);
        assert!(search("hamburg", &conn).is_empty());
        Ok(())
    }

    #[test]
    fn tag_value_search_skips_elements_without_coordinates() -> Result<()> {
        let conn = conn();
        // Inserted but never given lat/lon: `SearchedPlace::from` would panic on it.
        super::insert(
            &OverpassElement::mock_with_tags(1, &[("name", "Hamburg Cafe")]),
            &conn,
        )?;
        assert!(search("hamburg", &conn).is_empty());
        Ok(())
    }

    #[test]
    fn tag_value_search_skips_elements_without_tags() -> Result<()> {
        let conn = conn();
        let element = super::insert(&OverpassElement::mock(1), &conn)?;
        super::set_lat_lon(element.id, 53.5, 9.9, &conn)?;
        // `mock` has an empty tag map; must not blow up json_each.
        assert!(search("hamburg", &conn).is_empty());
        Ok(())
    }

    #[test]
    fn tag_value_search_requires_all_words() -> Result<()> {
        let conn = conn();
        let hit = insert_place(
            1,
            &[
                ("name", "Kaffeeklatsch"),
                ("addr:city", "Hamburg"),
                ("cuisine", "cafe"),
            ],
            53.5,
            9.9,
            &conn,
        );
        insert_place(
            2,
            &[("name", "Nordsee"), ("addr:city", "Hamburg")],
            53.6,
            9.8,
            &conn,
        );
        // Words may match different tags on the same element.
        assert_eq!(vec![hit], search("hamburg cafe", &conn));
        // A word that matches nothing kills the whole row.
        assert!(search("hamburg sushi", &conn).is_empty());
        Ok(())
    }

    #[test]
    fn tag_value_search_escapes_like_wildcards() -> Result<()> {
        let conn = conn();
        let hit = insert_place(1, &[("name", "100% Coffee")], 53.5, 9.9, &conn);
        insert_place(2, &[("name", "Nordsee")], 53.6, 9.8, &conn);
        // Unescaped, '%100%%' would match every row.
        assert_eq!(vec![hit], search("100%", &conn));
        Ok(())
    }

    #[test]
    fn tag_value_search_excludes_deleted() -> Result<()> {
        let conn = conn();
        let element = insert_place(1, &[("name", "Hamburg Cafe")], 53.5, 9.9, &conn);
        super::set_deleted_at(element.id, Some(OffsetDateTime::now_utc()), &conn)?;
        assert!(search("hamburg", &conn).is_empty());
        Ok(())
    }

    #[test]
    fn tag_value_search_ranks_name_above_other_tags() -> Result<()> {
        let conn = conn();
        let exact = insert_place(1, &[("name", "Hamburg")], 53.5, 9.9, &conn);
        let prefix = insert_place(2, &[("name", "Hamburger Grill")], 53.5, 9.9, &conn);
        let infix = insert_place(3, &[("name", "Cafe Hamburg Nord")], 53.5, 9.9, &conn);
        let other = insert_place(
            4,
            &[("name", "Nordsee"), ("addr:city", "Hamburg")],
            53.5,
            9.9,
            &conn,
        );
        assert_eq!(vec![exact, prefix, infix, other], search("hamburg", &conn));
        Ok(())
    }

    #[test]
    fn tag_value_search_breaks_rank_ties_by_distance() -> Result<()> {
        let conn = conn();
        let far = insert_place(
            1,
            &[("name", "Nordsee"), ("addr:city", "Hamburg")],
            60.0,
            9.9,
            &conn,
        );
        let near = insert_place(
            2,
            &[("name", "Kaffeeklatsch"), ("addr:city", "Hamburg")],
            53.6,
            9.9,
            &conn,
        );
        let ranked = super::select_by_tag_value_search("hamburg", Some((53.5, 9.9)), 100, &conn)?;
        let ids: Vec<i64> = ranked.into_iter().map(|it| it.element.id).collect();
        assert_eq!(vec![near.id, far.id], ids);
        Ok(())
    }

    #[test]
    fn tag_value_search_honours_row_limit() -> Result<()> {
        let conn = conn();
        for id in 1..=5 {
            insert_place(id, &[("name", &format!("Hamburg {id}"))], 53.5, 9.9, &conn);
        }
        assert_eq!(
            2,
            super::select_by_tag_value_search("hamburg", None, 2, &conn)?.len()
        );
        Ok(())
    }

    #[test]
    fn tag_value_search_ranks_by_localized_name() -> Result<()> {
        let conn = conn();
        // Raw name doesn't match "cafe", but the displayed name (name:en) is an
        // exact match. It must rank as exact (0), above a prefix hit, not be
        // demoted to an other-tag hit (3) because the raw name was consulted.
        let localized_exact = insert_place(
            1,
            &[("name", "Kaffee"), ("name:en", "Cafe")],
            53.5,
            9.9,
            &conn,
        );
        let prefix = insert_place(2, &[("name", "Cafe Central")], 53.6, 9.8, &conn);
        let ranked = super::select_by_tag_value_search("cafe", None, 100, &conn)?;
        let by_id: std::collections::HashMap<i64, i64> =
            ranked.iter().map(|r| (r.element.id, r.rank)).collect();
        assert_eq!(Some(&0), by_id.get(&localized_exact.id));
        assert_eq!(Some(&1), by_id.get(&prefix.id));
        // Exact localized match sorts first.
        assert_eq!(localized_exact.id, ranked[0].element.id);
        Ok(())
    }

    #[test]
    fn count_by_tag_value_search_counts_all_matches() -> Result<()> {
        let conn = conn();
        for id in 1..=5 {
            insert_place(id, &[("name", &format!("Hamburg {id}"))], 53.5, 9.9, &conn);
        }
        assert_eq!(5, super::count_by_tag_value_search("hamburg", &conn)?);
        Ok(())
    }
}
