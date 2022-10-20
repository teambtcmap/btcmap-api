use crate::model::Area;
use crate::model::DailyReport;
use crate::model::Element;
use crate::model::ElementEvent;
use crate::model::User;
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

pub static DAILY_REPORT_INSERT: &str = "INSERT INTO report (area_id, date, total_elements, total_elements_onchain, total_elements_lightning, total_elements_lightning_contactless, up_to_date_elements, outdated_elements, legacy_elements, elements_created, elements_updated, elements_deleted) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
pub static DAILY_REPORT_SELECT_ALL: &str = "SELECT area_id, date, total_elements, total_elements_onchain, total_elements_lightning, total_elements_lightning_contactless, up_to_date_elements, outdated_elements, legacy_elements, elements_created, elements_updated, elements_deleted, created_at, updated_at, deleted_at FROM report ORDER BY date DESC";
pub static DAILY_REPORT_SELECT_BY_AREA_ID_AND_DATE: &str = "SELECT area_id, date, total_elements, total_elements_onchain, total_elements_lightning, total_elements_lightning_contactless, up_to_date_elements, outdated_elements, legacy_elements, elements_created, elements_updated, elements_deleted, created_at, updated_at, deleted_at FROM report WHERE area_id = ? AND date = ?";
pub static DAILY_REPORT_UPDATE_EVENT_COUNTERS: &str = "UPDATE report SET elements_created = ?, elements_updated = ?, elements_deleted = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ') WHERE area_id = ? AND date = ?";

pub static AREA_INSERT: &str =
    "INSERT INTO area (id, name, type, min_lon, min_lat, max_lon, max_lat) VALUES (:id, '', '', 0, 0, 0, 0)";
pub static AREA_SELECT_ALL: &str =
    "SELECT id, name, type, min_lon, min_lat, max_lon, max_lat, tags, created_at, updated_at, deleted_at FROM area ORDER BY updated_at DESC";
pub static AREA_SELECT_BY_ID: &str =
    "SELECT id, name, type, min_lon, min_lat, max_lon, max_lat, tags, created_at, updated_at, deleted_at FROM area WHERE id = ?";
pub static AREA_SELECT_BY_NAME: &str = "SELECT id, name, type, min_lon, min_lat, max_lon, max_lat, tags, created_at, updated_at, deleted_at FROM area WHERE UPPER(name) = UPPER(?)";
pub static AREA_SELECT_UPDATED_SINCE: &str = "SELECT id, name, type, min_lon, min_lat, max_lon, max_lat, tags, created_at, updated_at, deleted_at FROM area WHERE updated_at > ? ORDER BY updated_at DESC";
pub static AREA_INSERT_TAG: &str = "UPDATE area SET tags = json_set(tags, :tag_name, :tag_value) where id = :area_id;";
pub static AREA_DELETE_TAG: &str = "UPDATE area SET tags = json_remove(tags, :tag_name) where id = :area_id;";

pub static ELEMENT_EVENT_INSERT: &str = "INSERT INTO event (date, element_id, element_lat, element_lon, element_name, type, user_id, user) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";
pub static ELEMENT_EVENT_SELECT_ALL: &str = "SELECT ROWID, date, element_id, element_lat, element_lon, element_name, type, user_id, user, created_at, updated_at, deleted_at FROM event ORDER BY date DESC";
pub static ELEMENT_EVENT_SELECT_BY_ID: &str = "SELECT ROWID, date, element_id, element_lat, element_lon, element_name, type, user_id, user, created_at, updated_at, deleted_at FROM event where ROWID = ?";
pub static ELEMENT_EVENT_SELECT_UPDATED_SINCE: &str = "SELECT ROWID, date, element_id, element_lat, element_lon, element_name, type, user_id, user, created_at, updated_at, deleted_at FROM event WHERE updated_at > ? ORDER BY date DESC";

pub static USER_INSERT: &str = "INSERT INTO user (id, data) VALUES (?, ?)";
pub static USER_SELECT_ALL: &str =
    "SELECT id, data, created_at, updated_at, deleted_at FROM user ORDER BY updated_at DESC";
pub static USER_SELECT_BY_ID: &str =
    "SELECT id, data, created_at, updated_at, deleted_at FROM user WHERE id = ?";
pub static USER_SELECT_UPDATED_SINCE: &str =
    "SELECT id, data, created_at, updated_at, deleted_at FROM user WHERE updated_at > ? ORDER BY updated_at DESC";

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
            area_id: row.get(0)?,
            date: row.get(1)?,
            total_elements: row.get(2)?,
            total_elements_onchain: row.get(3)?,
            total_elements_lightning: row.get(4)?,
            total_elements_lightning_contactless: row.get(5)?,
            up_to_date_elements: row.get(6)?,
            outdated_elements: row.get(7)?,
            legacy_elements: row.get(8)?,
            elements_created: row.get(9)?,
            elements_updated: row.get(10)?,
            elements_deleted: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
            deleted_at: row.get(14)?,
        })
    }
}

pub fn mapper_area_full() -> fn(&Row) -> rusqlite::Result<Area> {
    |row: &Row| -> rusqlite::Result<Area> {
        let tags: String = row.get(7)?;
        let tags: Value = serde_json::from_str(&tags).unwrap_or_default();

        Ok(Area {
            id: row.get(0)?,
            name: row.get(1)?,
            area_type: row.get(2)?,
            min_lon: row.get(3)?,
            min_lat: row.get(4)?,
            max_lon: row.get(5)?,
            max_lat: row.get(6)?,
            tags: tags,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            deleted_at: row.get(10)?,
        })
    }
}

pub fn mapper_element_event_full() -> fn(&Row) -> rusqlite::Result<ElementEvent> {
    |row: &Row| -> rusqlite::Result<ElementEvent> {
        Ok(ElementEvent {
            id: row.get(0)?,
            date: row.get(1)?,
            element_id: row.get(2)?,
            element_lat: row.get(3)?,
            element_lon: row.get(4)?,
            element_name: row.get(5)?,
            event_type: row.get(6)?,
            user_id: row.get(7)?,
            user: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
            deleted_at: row.get(11)?,
        })
    }
}

pub fn mapper_user_full() -> fn(&Row) -> rusqlite::Result<User> {
    |row: &Row| -> rusqlite::Result<User> {
        let data: String = row.get(1)?;
        let data: Value = serde_json::from_str(&data).unwrap_or_default();

        Ok(User {
            id: row.get(0)?,
            data,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
            deleted_at: row.get(4)?,
        })
    }
}
