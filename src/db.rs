use crate::area::Area;
use crate::daily_report::DailyReport;
use crate::element::Element;
use rusqlite::{Connection, Row};
use serde_json::Value;
use std::fs::remove_file;

pub fn cli_main(args: &[String], db_conn: Connection) {
    match args.first() {
        Some(first_arg) => match first_arg.as_str() {
            "migrate" => cli_migrate(db_conn),
            "drop" => cli_drop(db_conn),
            _ => panic!("Unknown action {first_arg}"),
        },
        None => {
            panic!("No db actions passed");
        }
    }
}

fn cli_migrate(db_conn: Connection) {
    let mut schema_ver: i16 = db_conn
        .query_row("SELECT user_version FROM pragma_user_version", [], |row| {
            row.get(0)
        })
        .unwrap();

    if schema_ver == 0 {
        println!("Creating database schema");
        db_conn
            .execute_batch(include_str!("../migrations/1.sql"))
            .unwrap();
        db_conn
            .execute_batch(&format!("PRAGMA user_version={}", 1))
            .unwrap();
        schema_ver += 1;
    }

    if schema_ver == 1 {
        println!("Migrating database schema to version 2");
        db_conn
            .execute_batch(include_str!("../migrations/2.sql"))
            .unwrap();
        db_conn
            .execute_batch(&format!("PRAGMA user_version={}", 2))
            .unwrap();
        schema_ver += 1;
    }

    if schema_ver == 2 {
        println!("Migrating database schema to version 3");
        db_conn
            .execute_batch(include_str!("../migrations/3.sql"))
            .unwrap();
        db_conn
            .execute_batch(&format!("PRAGMA user_version={}", 3))
            .unwrap();
        schema_ver += 1;
    }

    println!("Database schema is up to date (version {schema_ver})");
}

fn cli_drop(db_conn: Connection) {
    if !db_conn.path().unwrap().exists() {
        panic!("Database does not exist");
    } else {
        println!(
            "Found database at {}",
            db_conn.path().unwrap().to_str().unwrap()
        );
        remove_file(db_conn.path().unwrap()).unwrap();
        println!("Database file was removed");
    }
}

pub fn mapper_element_full() -> fn(&Row) -> rusqlite::Result<Element> {
    |row: &Row| -> rusqlite::Result<Element> {
        let data: String = row.get(1)?;
        let data: Value = serde_json::from_str(&data).unwrap_or_default();

        Ok(Element {
            id: row.get(0)?,
            data,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
            deleted_at: row.get(4)?,
        })
    }
}

pub fn mapper_daily_report_full() -> fn(&Row) -> rusqlite::Result<DailyReport> {
    |row: &Row| -> rusqlite::Result<DailyReport> {
        Ok(DailyReport {
            date: row.get(0)?,
            total_elements: row.get(1)?,
            up_to_date_elements: row.get(2)?,
            outdated_elements: row.get(3)?,
            legacy_elements: row.get(4)?,
            elements_created: row.get(5)?,
            elements_updated: row.get(6)?,
            elements_deleted: row.get(7)?,
        })
    }
}

pub fn mapper_area_full() -> fn(&Row) -> rusqlite::Result<Area> {
    |row: &Row| -> rusqlite::Result<Area> {
        Ok(Area {
            id: row.get(0)?,
            name: row.get(1)?,
            area_type: row.get(2)?,
            min_lon: row.get(3)?,
            min_lat: row.get(4)?,
            max_lon: row.get(5)?,
            max_lat: row.get(6)?,
        })
    }
}