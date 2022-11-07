use crate::model::Event;
use crate::model::Report;
use crate::model::User;
use include_dir::include_dir;
use include_dir::Dir;
use rusqlite::{Connection, Row};
use serde_json::Value;
use std::fs::remove_file;
#[cfg(test)]
use std::sync::atomic::AtomicUsize;

#[cfg(test)]
pub static COUNTER: AtomicUsize = AtomicUsize::new(1);

static MIGRATIONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/migrations");

pub static REPORT_INSERT: &str = r#"
    INSERT INTO report (
        area_id, 
        date,
        tags
    ) VALUES (
        :area_id,
        :date,
        :tags
    )
"#;

pub static REPORT_SELECT_ALL: &str = r#"
    SELECT 
        ROWID,
        area_id,
        date,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM report 
    ORDER BY updated_at
"#;

pub static REPORT_SELECT_UPDATED_SINCE: &str = r#"
    SELECT 
        ROWID,
        area_id,
        date,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM report 
    WHERE updated_at > :updated_since
    ORDER BY updated_at
"#;

pub static REPORT_SELECT_BY_ID: &str = r#"
    SELECT 
        ROWID, 
        area_id, 
        date, 
        tags, 
        created_at, 
        updated_at, 
        deleted_at 
    FROM report 
    WHERE ROWID = :id
"#;

pub static REPORT_SELECT_BY_AREA_ID_AND_DATE: &str = r#"
    SELECT 
        ROWID,
        area_id,
        date,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM report 
    WHERE area_id = :area_id AND date = :date
"#;

pub static EVENT_INSERT: &str = r#"
    INSERT INTO event (
        date, 
        element_id, 
        type, 
        user_id
    ) VALUES (
        :date,
        :element_id,
        :type,
        :user_id
    )
"#;

pub static EVENT_SELECT_ALL: &str = "SELECT ROWID, date, element_id, type, user_id, created_at, updated_at, deleted_at FROM event ORDER BY date DESC";
pub static EVENT_SELECT_BY_ID: &str = "SELECT ROWID, date, element_id, type, user_id, created_at, updated_at, deleted_at FROM event where ROWID = ?";
pub static EVENT_SELECT_UPDATED_SINCE: &str = "SELECT ROWID, date, element_id, type, user_id, created_at, updated_at, deleted_at FROM event WHERE updated_at > ? ORDER BY date DESC";

pub static USER_INSERT: &str = r#"
    INSERT INTO user (
        id,
        osm_json
    ) VALUES (
        :id,
        :osm_json
    )
"#;

pub static USER_SELECT_ALL: &str = r#"
    SELECT
        id,
        osm_json,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM user
    ORDER BY updated_at
"#;

pub static USER_SELECT_BY_ID: &str = r#"
    SELECT
        id,
        osm_json,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM user
    WHERE id = :id
"#;

pub static USER_SELECT_UPDATED_SINCE: &str = r#"
    SELECT
        id,
        osm_json,
        tags,
        created_at,
        updated_at,
        deleted_at
    FROM user
    WHERE updated_at > :updated_since
    ORDER BY updated_at
"#;

pub static USER_INSERT_TAG: &str = r#"
    UPDATE user
    SET tags = json_set(tags, :tag_name, :tag_value)
    WHERE id = :user_id
"#;

pub static USER_DELETE_TAG: &str = r#"
    UPDATE user
    SET tags = json_remove(tags, :tag_name)
    WHERE id = :user_id
"#;

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

pub fn mapper_report_full() -> fn(&Row) -> rusqlite::Result<Report> {
    |row: &Row| -> rusqlite::Result<Report> {
        let tags: String = row.get(3)?;
        let tags: Value = serde_json::from_str(&tags).unwrap_or_default();

        Ok(Report {
            id: row.get(0)?,
            area_id: row.get(1)?,
            date: row.get(2)?,
            tags: tags,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            deleted_at: row.get(6)?,
        })
    }
}

pub fn mapper_event_full() -> fn(&Row) -> rusqlite::Result<Event> {
    |row: &Row| -> rusqlite::Result<Event> {
        Ok(Event {
            id: row.get(0)?,
            date: row.get(1)?,
            element_id: row.get(2)?,
            r#type: row.get(3)?,
            user_id: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            deleted_at: row.get(7)?,
        })
    }
}

pub fn mapper_user_full() -> fn(&Row) -> rusqlite::Result<User> {
    |row: &Row| -> rusqlite::Result<User> {
        let osm_json: String = row.get(1)?;
        let osm_json: Value = serde_json::from_str(&osm_json).unwrap_or_default();

        let tags: String = row.get(2)?;
        let tags: Value = serde_json::from_str(&tags).unwrap_or_default();

        Ok(User {
            id: row.get(0)?,
            osm_json,
            tags,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
        })
    }
}
