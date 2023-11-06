use crate::error::Error;
use crate::Result;
use directories::ProjectDirs;
use include_dir::include_dir;
use include_dir::Dir;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::fmt;
use std::fs::create_dir_all;
use std::fs::remove_file;
use tracing::info;
use tracing::warn;

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

pub fn pool() -> Result<Pool<SqliteConnectionManager>> {
    let manager = SqliteConnectionManager::file(get_file_path()?).with_init(|conn| {
        conn.execute_batch(
            r#"
                PRAGMA journal_mode=WAL;
                PRAGMA synchronous=NORMAL;
            "#,
        )
    });
    Ok(Pool::builder().max_size(4).build(manager)?)
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
        warn!(%migration, "Found new migration");
        let tx = db.transaction()?;
        tx.execute_batch(&migration.1)?;
        tx.execute_batch(&format!("PRAGMA user_version={}", migration.0))?;
        tx.commit()?;
        schema_ver = migration.0;
    }

    info!(schema_ver, "Database schema is up to date");

    Ok(())
}

fn drop(db: Connection) -> Result<()> {
    remove_file(
        db.path()
            .ok_or(Error::Other("Failed to find database path".into()))?,
    )?;
    let db_path = db.path().unwrap();
    info!(?db_path, "Database file was removed");
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
pub mod tests {
    use super::*;

    #[test]
    fn run() -> Result<()> {
        let res = super::run(&[], Connection::open_in_memory()?);
        assert!(res.is_err());

        let res = super::run(&["test".into()], Connection::open_in_memory()?);
        assert!(res.is_err());

        let res = super::run(&["migrate".into()], Connection::open_in_memory()?);
        assert!(res.is_ok());

        let res = super::run(&["drop".into()], Connection::open_in_memory()?);
        assert!(res.is_err());

        Ok(())
    }

    #[test]
    fn run_migrations() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;

        let mut migrations = vec![Migration(1, "CREATE TABLE foo(bar);".into())];
        execute_migrations(&migrations, &mut conn)?;

        let schema_ver: i16 =
            conn.query_row("SELECT user_version FROM pragma_user_version", [], |row| {
                row.get(0)
            })?;

        assert_eq!(1, schema_ver);

        migrations.push(Migration(
            2,
            "INSERT INTO foo (bar) values ('qwerty');".into(),
        ));

        execute_migrations(&migrations, &mut conn)?;

        let schema_ver: i16 =
            conn.query_row("SELECT user_version FROM pragma_user_version", [], |row| {
                row.get(0)
            })?;

        assert_eq!(2, schema_ver);

        Ok(())
    }
}
