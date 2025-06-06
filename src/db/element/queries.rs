use super::schema::{self, Columns};
use crate::{element::Element, osm::overpass::OverpassElement, Result};
use rusqlite::{params, Connection};

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

#[cfg(test)]
mod test {
    use crate::Error;
    use crate::{osm::overpass::OverpassElement, test::mock_conn, Result};

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
}
