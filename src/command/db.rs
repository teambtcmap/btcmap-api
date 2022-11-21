use crate::error::Error;
use crate::Result;
use directories::ProjectDirs;
use include_dir::include_dir;
use include_dir::Dir;
use rusqlite::Connection;
use std::fmt;
use std::fs::create_dir_all;
use std::fs::remove_file;

static MIGRATIONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/migrations");

struct Migration(i16, String);

impl fmt::Display for Migration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}, {})",
            self.0,
            self.1
                .replace("\n", "")
                .replace("    ", "")
                .replace(";", "; "),
        )
    }
}

pub fn run(args: &[String], db: Connection) -> Result<()> {
    let first_arg = match args.first() {
        Some(some) => some,
        None => Err(Error::CLI("No DB actions passed".into()))?,
    };

    match first_arg.as_str() {
        "migrate" => {}
        "drop" => drop(db)?,
        _ => Err(Error::CLI(format!("Unknown command: {first_arg}")))?,
    }

    Ok(())
}

pub fn migrate(db: &mut Connection) -> Result<()> {
    execute_migrations(&get_migrations()?, db)
}

pub fn open_connection() -> Result<Connection> {
    let conn = Connection::open(get_file_path()?)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    Ok(conn)
}

pub fn get_file_path() -> Result<PathBuf> {
    let project_dirs = match ProjectDirs::from("org", "BTC Map", "BTC Map") {
        Some(project_dirs) => project_dirs,
        None => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Can't find home directory",
        ))?,
    };

    if !project_dirs.data_dir().exists() {
        create_dir_all(project_dirs.data_dir())?;
    }

    Ok(project_dirs.data_dir().join("btcmap.db"))
}

fn execute_migrations(migrations: &Vec<Migration>, db: &mut Connection) -> Result<()> {
    let mut schema_ver: i16 =
        db.query_row("SELECT user_version FROM pragma_user_version", [], |row| {
            row.get(0)
        })?;

    let new_migrations: Vec<&Migration> =
        migrations.iter().filter(|it| it.0 > schema_ver).collect();

    for migration in new_migrations {
        log::warn!("Found new migration: {migration}");
        let tx = db.transaction()?;
        tx.execute_batch(&migration.1)?;
        tx.execute_batch(&format!("PRAGMA user_version={}", migration.0))?;
        tx.commit()?;
        schema_ver = migration.0;
    }

    log::info!("Database schema is up to date (version {schema_ver})");

    Ok(())
}

fn drop(db: Connection) -> Result<()> {
    remove_file(
        db.path()
            .ok_or(Error::Other("Failed to find database path".into()))?,
    )?;
    log::info!("Database file was removed");
    Ok(())
}

fn get_migrations() -> Result<Vec<Migration>> {
    let mut index = 1;
    let mut res = vec![];

    loop {
        let file_name = format!("{index}.sql");
        let file = MIGRATIONS_DIR.get_file(&file_name);
        match file {
            Some(file) => {
                let sql = file.contents_utf8().ok_or(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Can't read {file_name} in UTF-8"),
                ))?;

                res.push(Migration(index, sql.to_string()));

                index += 1;
            }
            None => {
                break;
            }
        }
    }

    Ok(res)
}

use std::path::PathBuf;
#[cfg(test)]
use std::sync::atomic::AtomicUsize;

#[cfg(test)]
pub static COUNTER: AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use super::*;

    #[test]
    fn run_migrations() {
        let db_name = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut db =
            Connection::open(format!("file::testdb_{db_name}:?mode=memory&cache=shared")).unwrap();
        let mut migrations = vec![Migration(1, "CREATE TABLE foo(bar);".into())];
        let res = execute_migrations(&migrations, &mut db);
        assert!(res.is_ok());

        let schema_ver: i16 = db
            .query_row("SELECT user_version FROM pragma_user_version", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(1, schema_ver);

        migrations.push(Migration(
            2,
            "INSERT INTO foo (bar) values ('qwerty');".into(),
        ));

        let res = execute_migrations(&migrations, &mut db);
        assert!(res.is_ok());

        let schema_ver: i16 = db
            .query_row("SELECT user_version FROM pragma_user_version", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(2, schema_ver);
    }
}
