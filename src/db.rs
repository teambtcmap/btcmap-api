use crate::Result;
use include_dir::include_dir;
use include_dir::Dir;
use rusqlite::Connection;
use std::fs::remove_file;

static MIGRATIONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/migrations");

pub fn cli_main(args: &[String], mut db: Connection) -> Result<()> {
    match args.first() {
        Some(first_arg) => match first_arg.as_str() {
            "migrate" => migrate(&mut db)?,
            "drop" => drop(db)?,
            _ => panic!("Unknown action {first_arg}"),
        },
        None => {
            panic!("No db actions passed");
        }
    }

    Ok(())
}

pub fn migrate(db: &mut Connection) -> Result<()> {
    let mut schema_ver: i16 =
        db.query_row("SELECT user_version FROM pragma_user_version", [], |row| {
            row.get(0)
        })?;

    loop {
        let file_name = format!("{}.sql", schema_ver + 1);
        let file = MIGRATIONS_DIR.get_file(&file_name);
        match file {
            Some(file) => {
                log::warn!("Found new migration: {file_name}");
                let sql = file.contents_utf8().ok_or(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Can't read {file_name} in UTF-8"),
                ))?;
                log::warn!("Executing query:\n{sql}");
                let tx = db.transaction()?;
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

fn drop(db: Connection) -> Result<()> {
    remove_file(db.path().unwrap())?;
    log::info!("Database file was removed");
    Ok(())
}

#[cfg(test)]
use std::sync::atomic::AtomicUsize;

#[cfg(test)]
pub static COUNTER: AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
