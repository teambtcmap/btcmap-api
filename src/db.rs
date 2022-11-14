use include_dir::include_dir;
use include_dir::Dir;
use rusqlite::Connection;
use std::error::Error;
use std::fs::remove_file;

static MIGRATIONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/migrations");

pub fn cli_main(args: &[String], mut db: Connection) {
    match args.first() {
        Some(first_arg) => match first_arg.as_str() {
            "migrate" => {
                if let Err(err) = migrate(&mut db) {
                    log::error!("Migration faied: {err}");
                    std::process::exit(1);
                }
            }
            "drop" => drop(db),
            _ => panic!("Unknown action {first_arg}"),
        },
        None => {
            panic!("No db actions passed");
        }
    }
}

pub fn migrate(db: &mut Connection) -> Result<(), Box<dyn Error>> {
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
                let sql = file
                    .contents_utf8()
                    .ok_or(format!("Can't read {file_name} in UTF-8"))?;
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

fn drop(db: Connection) {
    if !db.path().unwrap().exists() {
        log::error!("Database does not exist");
        std::process::exit(1);
    } else {
        log::info!("Found database at {}", db.path().unwrap().to_str().unwrap());
        remove_file(db.path().unwrap()).unwrap();
        log::info!("Database file was removed");
    }
}

#[cfg(test)]
use std::sync::atomic::AtomicUsize;

#[cfg(test)]
pub static COUNTER: AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
