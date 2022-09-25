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
    let schema_ver: i16 = db_conn
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
    } else {
        println!("Found database schema version {}", schema_ver);
    }

    println!("Database schema is up to date");
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
