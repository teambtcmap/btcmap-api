use rusqlite::Row;
use std::sync::OnceLock;

pub const TABLE_NAME: &str = "place_import_origin";

#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    Id,
    Name,
    GiteaSyncEnabled,
    GiteaLabelId,
}

#[derive(PartialEq, Eq, Debug)]
pub struct ImportOrigin {
    pub id: i64,
    pub name: String,
    pub gitea_sync_enabled: bool,
    pub gitea_label_id: Option<i64>,
}

impl ImportOrigin {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::Name,
                Columns::GiteaSyncEnabled,
                Columns::GiteaLabelId,
            ]
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<ImportOrigin> {
        |row| {
            Ok(ImportOrigin {
                id: row.get(Columns::Id.as_ref())?,
                name: row.get(Columns::Name.as_ref())?,
                gitea_sync_enabled: row.get(Columns::GiteaSyncEnabled.as_ref())?,
                gitea_label_id: row.get(Columns::GiteaLabelId.as_ref())?,
            })
        }
    }
}
