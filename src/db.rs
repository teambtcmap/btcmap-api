use crate::Result;
use deadpool_sqlite::Config;
use deadpool_sqlite::Hook;
use deadpool_sqlite::Pool;
use deadpool_sqlite::Runtime;
use directories::ProjectDirs;
use include_dir::include_dir;
use include_dir::Dir;
use rusqlite::Connection;
use std::fmt;
use std::fs::create_dir_all;
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

pub fn migrate(db: &mut Connection) -> Result<()> {
    execute_migrations(&get_migrations()?, db)
}

pub fn pool() -> Result<Pool> {
    Ok(Config::new(get_file_path()?)
        .builder(Runtime::Tokio1)?
        .post_create(Hook::Fn(Box::new(|conn, _| {
            let conn = conn.lock().unwrap();
            conn.pragma_update(None, "journal_mode", "WAL").unwrap();
            conn.pragma_update(None, "synchronous", "NORMAL").unwrap();
            conn.pragma_update(None, "foreign_keys", "ON").unwrap();
            // > The default suggested cache size is -2000, which means the cache size is limited to 2048000 bytes of memory
            // Source: https://www.sqlite.org/pragma.html#pragma_cache_size
            // The default page size is 4096 bytes, cache_size sets the number of pages
            // conn.pragma_update(None, "cache_size", 25000).unwrap();
            Ok(())
        })))
        .build()?)
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
pub mod test {
    use rusqlite::Connection;

    use crate::Result;

    #[test]
    fn run_migrations() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        let mut migrations = vec![super::Migration(1, "CREATE TABLE foo(bar);".into())];
        super::execute_migrations(&migrations, &mut conn)?;
        let schema_ver: i16 =
            conn.query_row("SELECT user_version FROM pragma_user_version", [], |row| {
                row.get(0)
            })?;
        assert_eq!(1, schema_ver);
        migrations.push(super::Migration(
            2,
            "INSERT INTO foo (bar) values ('qwerty');".into(),
        ));
        super::execute_migrations(&migrations, &mut conn)?;
        let schema_ver: i16 =
            conn.query_row("SELECT user_version FROM pragma_user_version", [], |row| {
                row.get(0)
            })?;
        assert_eq!(2, schema_ver);
        Ok(())
    }
}
