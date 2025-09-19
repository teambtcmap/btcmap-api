use super::schema::{self, Columns, Element};
use crate::{service::overpass::OverpassElement, Result};
use rusqlite::{named_params, params, Connection};
use serde_json::{Map, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn insert(overpass_data: &OverpassElement, conn: &Connection) -> Result<Element> {
    let sql = format!(
        r#"
            INSERT INTO {table} ({overpass_data}) 
            VALUES (json(?1))
            RETURNING {projection}
        "#,
        table = schema::TABLE_NAME,
        overpass_data = Columns::OverpassData.as_str(),
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
            updated_at = Columns::UpdatedAt.as_str(),
            id = Columns::Id.as_str(),
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
        overpass_data = Columns::OverpassData.as_str(),
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
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
        lat = Columns::Lat.as_str(),
        lon = Columns::Lon.as_str(),
        deleted_at = Columns::DeletedAt.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(named_params! { ":min_lat": min_lat, ":max_lat": max_lat, ":min_lon": min_lon, ":max_lon": max_lon }, Element::mapper())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn select_by_payment_provider(
    payment_provider: &str,
    conn: &Connection,
) -> Result<Vec<Element>> {
    let sql = format!(
        r#"
            SELECT {projection}
            FROM {table}
            WHERE json_extract({overpass_data}, '$.tags.payment:{payment_provider}') = 'yes' AND deleted_at IS NULL
        "#,
        projection = Element::projection(),
        table = schema::TABLE_NAME,
        overpass_data = Columns::OverpassData.as_str(),
    );
    conn.prepare(&sql)?
        .query_map(params![], Element::mapper())?
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
        overpass_data = Columns::OverpassData.as_str(),
        id = Columns::Id.as_str(),
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
        tags = Columns::Tags.as_str(),
        id = Columns::Id.as_str(),
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
        tags = Columns::Tags.as_str(),
        id = Columns::Id.as_str(),
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
        lat = Columns::Lat.as_str(),
        lon = Columns::Lon.as_str(),
        id = Columns::Id.as_str(),
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
        updated_at = Columns::UpdatedAt.as_str(),
        id = Columns::Id.as_str(),
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
                deleted_at = Columns::DeletedAt.as_str(),
                id = Columns::Id.as_str(),
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
                deleted_at = Columns::DeletedAt.as_str(),
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
}
