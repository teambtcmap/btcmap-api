pub mod access_token;
pub mod area;
pub mod area_element;
pub mod ban;

use super::Migration;
use crate::Result;
use deadpool_sqlite::{Config, Hook, Pool, Runtime};
use include_dir::include_dir;
use include_dir::Dir;
use rusqlite::Connection;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Clone)]
pub struct MainPool(Arc<Pool>);

impl MainPool {
    pub fn new(pool: Pool) -> Self {
        Self(Arc::new(pool))
    }
}

impl std::ops::Deref for MainPool {
    type Target = Pool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn pool() -> Result<MainPool> {
    let pool_size = std::thread::available_parallelism()
        .map(|n| n.get() * 2)
        .unwrap_or(8);
    Config::new(super::db_file_path("btcmap.db")?)
        .builder(Runtime::Tokio1)?
        .max_size(pool_size)
        .post_create(Hook::Fn(Box::new(|conn, _| {
            let mut conn = conn.lock().unwrap();
            super::configure_connection(&conn);
            run_migrations(&mut conn).unwrap();
            Ok(())
        })))
        .build()
        .map_err(Into::into)
        .map(MainPool::new)
}

static MIGRATIONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/db/main/migrations");

fn run_migrations(conn: &mut Connection) -> Result<()> {
    let migrations = get_migrations()?;
    info!(migrations = migrations.len(), "loaded main db migrations");

    let mut schema_ver: i16 =
        conn.query_row("SELECT user_version FROM pragma_user_version", [], |row| {
            row.get(0)
        })?;

    let pending_migrations: Vec<&Migration> =
        migrations.iter().filter(|it| it.0 > schema_ver).collect();

    if !pending_migrations.is_empty() {
        info!(
            pending_migrations = migrations.len(),
            "found pending main db migrations"
        );
    }

    for migration in pending_migrations {
        warn!(%migration, "applying pending main db migration");
        let tx = conn.transaction()?;
        tx.execute_batch(&migration.1)?;
        tx.execute_batch(&format!("PRAGMA user_version={}", migration.0))?;
        tx.commit()?;
        schema_ver = migration.0;
    }

    info!(schema_ver, "main db schema is up to date");

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
                let sql = file.contents_utf8().ok_or_else(|| {
                    std::io::Error::other(format!("Can't read {file_name} in UTF-8"))
                })?;

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

#[cfg(test)]
pub mod test {
    use crate::db::main::run_migrations;

    use super::MainPool;
    use deadpool_sqlite::{Config, Hook, Runtime};
    use rusqlite::Connection;

    pub fn pool() -> MainPool {
        let pool_size = std::thread::available_parallelism()
            .map(|n| n.get() * 2)
            .unwrap_or(8);
        let inner = Config::new(":memory:")
            .builder(Runtime::Tokio1)
            .unwrap()
            .max_size(pool_size)
            .post_create(Hook::Fn(Box::new(|conn, _| {
                let conn = conn.lock().unwrap();
                conn.execute_batch(include_str!("schema.sql")).unwrap();
                Ok(())
            })))
            .build()
            .unwrap();
        MainPool::new(inner)
    }

    pub fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("schema.sql")).unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn
    }

    #[test]
    fn migrations() {
        let mut conn = Connection::open_in_memory().unwrap();
        run_migrations(&mut conn).unwrap()
    }
}
