mod migrations;
pub mod og;

use crate::{service::filesystem::data_dir_file_path, Result};
use deadpool_sqlite::{Config, Hook, Pool, Runtime};
use std::sync::Arc;

#[derive(Clone)]
pub struct ImagePool(Arc<Pool>);

impl ImagePool {
    pub fn new(pool: Pool) -> Self {
        Self(Arc::new(pool))
    }
}

impl std::ops::Deref for ImagePool {
    type Target = Pool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn pool() -> Result<ImagePool> {
    let pool_size = std::thread::available_parallelism()
        .map(|n| n.get() * 2)
        .unwrap_or(8);
    let config = Config::new(data_dir_file_path("images.db")?)
        .builder(Runtime::Tokio1)?
        .max_size(pool_size)
        .post_create(Hook::Fn(Box::new(|conn, _| {
            let conn = conn.lock().unwrap();
            crate::db::configure_connection(&conn);
            migrations::v0_to_v1(&conn).unwrap();
            migrations::v1_to_v2(&conn).unwrap();
            Ok(())
        })));
    let pool = config.build()?;
    Ok(ImagePool::new(pool))
}
