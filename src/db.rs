use crate::model::ElementEvent;
use crate::model::Area;
use crate::model::DailyReport;
use crate::model::Element;
use include_dir::include_dir;
use include_dir::Dir;
use rusqlite::{Connection, Row};
use serde_json::Value;
use std::fs::remove_file;

static MIGRATIONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/migrations");

pub static ELEMENT_SELECT_ALL: &str = "SELECT * FROM element ORDER BY updated_at DESC";
pub static ELEMENT_SELECT_BY_ID: &str = "SELECT * FROM element WHERE id = ?";
pub static ELEMENT_SELECT_UPDATED_SINCE: &str =
    "SELECT * FROM element WHERE updated_at > ? ORDER BY updated_at DESC";

pub static DAILY_REPORT_INSERT: &str = "INSERT INTO daily_report (date, total_elements, total_elements_onchain, total_elements_lightning, total_elements_lightning_contactless, up_to_date_elements, outdated_elements, legacy_elements, elements_created, elements_updated, elements_deleted) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
pub static DAILY_REPORT_SELECT_ALL: &str = "SELECT date, total_elements, total_elements_onchain, total_elements_lightning, total_elements_lightning_contactless, up_to_date_elements, outdated_elements, legacy_elements, elements_created, elements_updated, elements_deleted FROM daily_report ORDER BY date DESC";
pub static DAILY_REPORT_SELECT_BY_DATE: &str = "SELECT date, total_elements, total_elements_onchain, total_elements_lightning, total_elements_lightning_contactless, up_to_date_elements, outdated_elements, legacy_elements, elements_created, elements_updated, elements_deleted FROM daily_report WHERE date = ?";
pub static DAILY_REPORT_DELETE_BY_DATE: &str = "DELETE FROM daily_report WHERE date = ?";

pub static AREA_SELECT_BY_ID: &str = "SELECT id, name, type, min_lon, min_lat, max_lon, max_lat FROM area WHERE id = ?";
pub static AREA_SELECT_ALL: &str = "SELECT id, name, type, min_lon, min_lat, max_lon, max_lat FROM area ORDER BY name";

pub static ELEMENT_EVENT_INSERT: &str = "INSERT INTO element_event (date, element_id, element_name, type, user) VALUES (?, ?, ?, ?, ?)";
pub static ELEMENT_EVENT_SELECT_ALL: &str = "SELECT date, element_id, element_name, type, user FROM element_event ORDER BY date DESC";

pub fn cli_main(args: &[String], mut db_conn: Connection) {
    match args.first() {
        Some(first_arg) => match first_arg.as_str() {
            "migrate" => {
                if let Err(err) = migrate(&mut db_conn) {
                    log::error!("Migration faied: {err}");
                    std::process::exit(1);
                }
            }
            "drop" => drop(db_conn),
            _ => panic!("Unknown action {first_arg}"),
        },
        None => {
            panic!("No db actions passed");
        }
    }
}

pub fn migrate(db_conn: &mut Connection) -> Result<(), Box<dyn std::error::Error>> {
    let mut schema_ver: i16 =
        db_conn.query_row("SELECT user_version FROM pragma_user_version", [], |row| {
            row.get(0)
        })?;

    loop {
        let file_name = format!("{}.sql", schema_ver + 1);
        let file = MIGRATIONS_DIR.get_file(&file_name);
        match file {
            Some(file) => {
                log::warn!("Found new migration: {file_name}");
                let sql = file
                    .contents_utf8()
                    .ok_or(format!("Can't read {file_name} in UTF-8"))?;
                log::warn!("{sql}");
                let tx = db_conn.transaction()?;
                tx.execute_batch(sql)?;
                tx.execute_batch(&format!("PRAGMA user_version={}", schema_ver + 1))?;
                tx.commit()?;
                schema_ver += 1;
            }
            None => {
                break;
            }
        }
    }

    log::info!("Database schema is up to date (version {schema_ver})");

    Ok(())
}

fn drop(db_conn: Connection) {
    if !db_conn.path().unwrap().exists() {
        log::error!("Database does not exist");
        std::process::exit(1);
    } else {
        log::info!(
            "Found database at {}",
            db_conn.path().unwrap().to_str().unwrap()
        );
        remove_file(db_conn.path().unwrap()).unwrap();
        log::info!("Database file was removed");
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
            total_elements_onchain: row.get(2)?,
            total_elements_lightning: row.get(3)?,
            total_elements_lightning_contactless: row.get(4)?,
            up_to_date_elements: row.get(5)?,
            outdated_elements: row.get(6)?,
            legacy_elements: row.get(7)?,
            elements_created: row.get(8)?,
            elements_updated: row.get(9)?,
            elements_deleted: row.get(10)?,
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

pub fn mapper_element_event_full() -> fn(&Row) -> rusqlite::Result<ElementEvent> {
    |row: &Row| -> rusqlite::Result<ElementEvent> {
        Ok(ElementEvent {
            date: row.get(0)?,
            element_id: row.get(1)?,
            element_name: row.get(2)?,
            event_type: row.get(3)?,
            user: row.get(4)?,
        })
    }
}