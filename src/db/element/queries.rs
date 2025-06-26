use super::schema::{self, Columns};
use crate::{element::Element, osm::overpass::OverpassElement, Result};
use rusqlite::{named_params, params, Connection};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn insert(overpass_data: &OverpassElement, conn: &Connection) -> Result<Element> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({overpass_data}) 
            VALUES (json(?1))
        "#,
        table = schema::TABLE_NAME,
        overpass_data = Columns::OverpassData.as_str(),
    );
    conn.execute(&sql, params![serde_json::to_string(overpass_data)?])?;
    select_by_id(conn.last_insert_rowid(), conn)
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
                WHERE {updated_at} > :updated_since
                ORDER BY {updated_at}, {id}
                LIMIT :limit
            "#,
            projection = Element::projection(),
            table = schema::TABLE_NAME,
            updated_at = Columns::UpdatedAt.as_str(),
            id = Columns::Id.as_str(),
        )
    } else {
        format!(
            r#"
                SELECT {projection}
                FROM {table}
                WHERE {deleted_at} IS NULL AND {updated_at} > :updated_since
                ORDER BY {updated_at}, {id}
                LIMIT :limit
            "#,
            projection = Element::projection(),
            table = schema::TABLE_NAME,
            deleted_at = Columns::DeletedAt.as_str(),
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
            Element::mapper(),
        )?
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn select_by_search_query(
    search_query: impl Into<String>,
    conn: &Connection,
) -> Result<Vec<Element>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE LOWER(json_extract({overpass_data}, '$.tags.name')) LIKE '%' || UPPER(?1) || '%'
            ORDER BY {updated_at}, {id}
        "#,
        projection = Element::projection(),
        table = schema::TABLE_NAME,
        overpass_data = Columns::OverpassData.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(params![search_query.into()], Element::mapper())?
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
            let osm_id = parts[1].parse::<i64>().unwrap();
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
        id = Columns::Id.as_str(),
    );
    conn.query_row(&sql, params![id], Element::mapper())
        .map_err(Into::into)
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
        overpass_data = Columns::OverpassData.as_str(),
    );
    Ok(conn.query_row(&sql, params![osm_type, osm_id], Element::mapper())?)
}

#[cfg(test)]
mod test {
    use crate::element::Element;
    use crate::Error;
    use crate::{osm::overpass::OverpassElement, test::mock_conn, Result};
    use time::macros::datetime;
    use time::OffsetDateTime;

    #[test]
    fn insert() -> Result<()> {
        let conn = mock_conn();
        let overpass_data = OverpassElement::mock(1);
        let element = super::insert(&overpass_data, &conn)?;
        assert_eq!(overpass_data, element.overpass_data);
        let element = super::select_by_id(1, &conn)?;
        assert_eq!(overpass_data, element.overpass_data);
        Ok(())
    }

    #[test]
    fn select_updated_since() -> Result<()> {
        let conn = mock_conn();
        let _element_1 = super::insert(&OverpassElement::mock(1), &conn)?
            .set_updated_at(&datetime!(2023-10-01 00:00 UTC), &conn)?;
        let element_2 = super::insert(&OverpassElement::mock(2), &conn)?
            .set_updated_at(&datetime!(2023-10-02 00:00 UTC), &conn)?;
        assert_eq!(
            vec![element_2.clone()],
            super::select_updated_since(datetime!(2023-10-01 00:00 UTC), None, false, &conn)?
        );
        Element::set_deleted_at(element_2.id, Some(OffsetDateTime::now_utc()), &conn)?;
        assert_eq!(
            0,
            super::select_updated_since(datetime!(2023-10-01 00:00 UTC), None, false, &conn)?.len()
        );
        Ok(())
    }

    #[test]
    fn select_by_search_query() -> Result<()> {
        let conn = mock_conn();

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
            super::select_by_search_query("downtown", &conn)?
        );

        // Test partial matching
        assert_eq!(
            vec![element2.clone()],
            super::select_by_search_query("park", &conn)?
        );

        // Test no matches
        assert_eq!(
            0,
            super::select_by_search_query("nonexistent", &conn)?.len()
        );

        // Test empty query (should match all named elements)
        let results = super::select_by_search_query("", &conn)?;
        assert_eq!(3, results.len());
        assert!(results.contains(&element1));
        assert!(results.contains(&element2));
        assert!(results.contains(&element3));

        Ok(())
    }

    #[test]
    fn select_by_search_query_special_chars() -> Result<()> {
        let conn = mock_conn();

        let element = super::insert(
            &OverpassElement::mock_with_tag(1, "name", "Café 'Le Paris'"),
            &conn,
        )?;

        // Test with special characters
        assert_eq!(
            vec![element.clone()],
            super::select_by_search_query("café", &conn)?
        );

        assert_eq!(
            vec![element.clone()],
            super::select_by_search_query("paris", &conn)?
        );

        // Test with single quote
        assert_eq!(
            vec![element],
            super::select_by_search_query("le paris", &conn)?
        );

        Ok(())
    }

    #[test]
    fn select_by_id() -> Result<()> {
        let conn = mock_conn();
        let element = super::insert(&OverpassElement::mock(1), &conn)?;
        assert_eq!(element, super::select_by_id(element.id, &conn)?);
        Ok(())
    }

    #[test]
    fn select_by_id_found() -> Result<()> {
        let conn = mock_conn();

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
            super::select_by_id(1, &mock_conn()),
            Err(Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows)),
        ));
    }

    #[test]
    fn select_by_osm_type_and_id() -> Result<()> {
        let conn = mock_conn();
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
}
