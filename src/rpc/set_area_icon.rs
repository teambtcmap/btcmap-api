use crate::{
    db::{self, area::schema::Area},
    Result,
};
use base64::prelude::*;
use deadpool_sqlite::Pool;
use geojson::JsonObject;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use std::{fs::OpenOptions, io::Write};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    pub id: String,
    pub icon_base64: String,
    pub icon_ext: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct Res {
    pub id: i64,
    pub tags: JsonObject,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl From<Area> for Res {
    fn from(val: Area) -> Self {
        Res {
            id: val.id,
            tags: val.tags,
            created_at: val.created_at,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
        }
    }
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let area = db::area::queries::select_by_id_or_alias(&params.id, pool).await?;
    let file_name = format!("{}.{}", area.id, params.icon_ext);
    let bytes = BASE64_STANDARD.decode(params.icon_base64)?;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(format!(
            "/srv/http/static.btcmap.org/images/areas/{file_name}"
        ))?;
    file.write_all(&bytes)?;
    file.flush()?;
    let url = format!("https://static.btcmap.org/images/areas/{file_name}");
    let patch_set = Map::from_iter([("icon:square".into(), url.into())].into_iter());
    let area = db::area::queries::patch_tags(area.id, patch_set, pool).await?;
    Ok(area.into())
}
