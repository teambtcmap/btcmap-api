use crate::model::Area;
use crate::model::Event;
use crate::model::Report;
use crate::model::area;
use crate::model::element;
use crate::model::Element;
use crate::Connection;
use crate::Result;
use crate::model::event;
use crate::model::report;
use rusqlite::named_params;
use tracing::info;
use tracing::warn;

pub fn run(db: Connection) -> Result<i128> {
    let mut duplicates = 0;
    duplicates += deduplicate_elements(&db)?;
    duplicates += deduplicate_events(&db)?;
    duplicates += deduplicate_areas(&db)?;
    duplicates += deduplicate_reports(&db)?;
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

fn deduplicate_events(db: &Connection) -> Result<i128> {
    info!("Looking for duplicate events");

    let mut duplicates = 0;

    loop {
        match find_duplicate_events(db)? {
            Some((e1, _)) => {
                db.execute(event::TOUCH, named_params! { ":id": e1.id })?;
                duplicates += 1;
            }
            None => break,
        }
    }

    Ok(duplicates)
}

fn find_duplicate_events(conn: &Connection) -> Result<Option<(Event, Event)>> {
    let events: Vec<Event> = conn
        .prepare(event::SELECT_ALL)?
        .query_map(
            named_params! { ":limit": std::i32::MAX },
            event::SELECT_ALL_MAPPER,
        )?
        .collect::<Result<Vec<Event>, _>>()?
        .into_iter()
        .collect();

    for e1 in &events {
        for e2 in &events {
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

fn deduplicate_areas(db: &Connection) -> Result<i128> {
    info!("Looking for duplicate areas");

    let mut duplicates = 0;

    loop {
        match find_duplicate_areas(db)? {
            Some((e1, _)) => {
                db.execute(area::TOUCH, named_params! { ":id": e1.id })?;
                duplicates += 1;
            }
            None => break,
        }
    }

    Ok(duplicates)
}

fn find_duplicate_areas(conn: &Connection) -> Result<Option<(Area, Area)>> {
    let areas: Vec<Area> = conn
        .prepare(area::SELECT_ALL)?
        .query_map(
            named_params! { ":limit": std::i32::MAX },
            area::SELECT_ALL_MAPPER,
        )?
        .collect::<Result<Vec<Area>, _>>()?
        .into_iter()
        .collect();

    for a1 in &areas {
        for a2 in &areas {
            if a1.id == a2.id {
                continue;
            }

            if a1.updated_at == a2.updated_at {
                warn!(
                    a1.id,
                    a2.id, a1.updated_at, a2.updated_at, "Found a duplicate updated_at",
                );

                return Ok(Some((a1.clone(), a2.clone())));
            }
        }
    }

    Ok(None)
}

fn deduplicate_reports(db: &Connection) -> Result<i128> {
    info!("Looking for duplicate reports");

    let mut duplicates = 0;

    loop {
        match find_duplicate_reports(db)? {
            Some((r1, _)) => {
                db.execute(report::TOUCH, named_params! { ":id": r1.id })?;
                duplicates += 1;
            }
            None => break,
        }
    }

    Ok(duplicates)
}

fn find_duplicate_reports(conn: &Connection) -> Result<Option<(Report, Report)>> {
    let reports: Vec<Report> = conn
        .prepare(report::SELECT_ALL)?
        .query_map(
            named_params! { ":limit": std::i32::MAX },
            report::SELECT_ALL_MAPPER,
        )?
        .collect::<Result<Vec<Report>, _>>()?
        .into_iter()
        .collect();

    for r1 in &reports {
        for r2 in &reports {
            if r1.id == r2.id {
                continue;
            }

            if r1.updated_at == r2.updated_at {
                warn!(
                    r1.id,
                    r2.id, r1.updated_at, r2.updated_at, "Found a duplicate updated_at",
                );

                return Ok(Some((r1.clone(), r2.clone())));
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

    #[traced_test]
    #[test]
    #[ignore]
    fn deduplicate_events() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        let insert = r#"
            INSERT INTO event (
                user_id,
                element_id,
                type,
                updated_at
            ) VALUES (
                :user_id,
                :element_id,
                :type,
                :updated_at
            )
        "#;

        conn.execute(
            insert,
            named_params! {
                ":user_id": 1,
                ":element_id": "node:1",
                ":type": "test",
                ":updated_at": "2023-05-05",
            },
        )?;

        conn.execute(
            insert,
            named_params! {
                ":user_id": 1,
                ":element_id": "node:1",
                ":type": "test",
                ":updated_at": "2023-05-05",
            },
        )?;

        let duplicates = super::run(conn).unwrap();
        assert_eq!(1, duplicates);

        Ok(())
    }

    #[traced_test]
    #[test]
    #[ignore]
    fn deduplicate_areas() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        let insert = r#"
            INSERT INTO area (
                id,
                updated_at
            ) VALUES (
                :id,
                :updated_at
            )
        "#;

        conn.execute(
            insert,
            named_params! {
                ":id": "test1",
                ":updated_at": "2023-05-05",
            },
        )?;

        conn.execute(
            insert,
            named_params! {
                ":id": "test2",
                ":updated_at": "2023-05-05",
            },
        )?;

        let duplicates = super::run(conn).unwrap();
        assert_eq!(1, duplicates);

        Ok(())
    }

    #[traced_test]
    #[test]
    #[ignore]
    fn deduplicate_reports() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        db::migrate(&mut conn)?;

        let insert = r#"
            INSERT INTO report (
                area_id,
                date,
                updated_at
            ) VALUES (
                :area_id,
                :date,
                :updated_at
            )
        "#;

        conn.execute(
            insert,
            named_params! {
                ":area_id": "test1",
                ":date": "2023-05-05",
                ":updated_at": "2023-05-05",
            },
        )?;

        conn.execute(
            insert,
            named_params! {
                ":area_id": "test2",
                ":date": "2023-05-05",
                ":updated_at": "2023-05-05",
            },
        )?;

        let duplicates = super::run(conn).unwrap();
        assert_eq!(1, duplicates);

        Ok(())
    }
}
