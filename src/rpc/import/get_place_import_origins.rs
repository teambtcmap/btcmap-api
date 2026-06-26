use crate::{
    db::{self, main::place_import_origin::schema::ImportOrigin},
    Result,
};
use deadpool_sqlite::Pool;
use serde::Serialize;

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub gitea_sync_enabled: bool,
    pub gitea_label_id: Option<i64>,
}

impl From<ImportOrigin> for Res {
    fn from(origin: ImportOrigin) -> Self {
        Res {
            name: origin.name,
            gitea_sync_enabled: origin.gitea_sync_enabled,
            gitea_label_id: origin.gitea_label_id,
        }
    }
}

pub async fn run(pool: &Pool) -> Result<Vec<Res>> {
    let origins = db::main::place_import_origin::queries::select_all(pool).await?;
    Ok(origins.into_iter().map(Into::into).collect())
}
