use crate::model::element;
use crate::model::Element;
use crate::Connection;
use crate::Result;
use rusqlite::named_params;
use tracing::info;
use tracing::warn;

pub fn run(db: Connection) -> Result<i128> {
    let mut duplicates = 0;
    duplicates += deduplicate_elements(&db)?;
    warn!(duplicates, "Removed duplicate updated_at");
    Ok(duplicates)
}

fn deduplicate_elements(db: &Connection) -> Result<i128> {
    info!("Looking for duplicate elements");

    let mut duplicates = 0;

    loop {
        match find_duplicate_elements(db)? {
            Some((e1, _)) => {
                db.execute(element::TOUCH, named_params! { ":id": e1.id })?;
                duplicates += 1;
            }
            None => break,
        }
    }

    Ok(duplicates)
}

fn find_duplicate_elements(conn: &Connection) -> Result<Option<(Element, Element)>> {
    let elements: Vec<Element> = conn
        .prepare(element::SELECT_ALL)?
        .query_map(
            named_params! { ":limit": std::i32::MAX },
            element::SELECT_ALL_MAPPER,
        )?
        .collect::<Result<Vec<Element>, _>>()?
        .into_iter()
        .collect();

    for e1 in &elements {
        for e2 in &elements {
            if e1.id == e2.id {
                continue;
            }

            if e1.updated_at == e2.updated_at {
                warn!(
                    e1.id,
                    e2.id, e1.updated_at, e2.updated_at, "Found a duplicate updated_at",
                );

                return Ok(Some((e1.clone(), e2.clone())));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use rusqlite::{named_params, Connection};
    use tracing_test::traced_test;

    use crate::command::db;

    use crate::Result;

    #[traced_test]
    #[test]
    #[ignore]
    fn deduplicate_elements() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        let insert = r#"
            INSERT INTO element (
                id,
                osm_json,
                updated_at
            ) VALUES (
                :id,
                :osm_json,
                :updated_at
            )
        "#;

        conn.execute(
            insert,
            named_params! {
                ":id": "1",
                ":osm_json": "{}",
                ":updated_at": "2023-05-05",
            },
        )?;

        conn.execute(
            insert,
            named_params! {
                ":id": "2",
                ":osm_json": "{}",
                ":updated_at": "2023-05-05",
            },
        )?;

        let duplicates = super::run(conn).unwrap();
        assert_eq!(1, duplicates);

        Ok(())
    }
}
