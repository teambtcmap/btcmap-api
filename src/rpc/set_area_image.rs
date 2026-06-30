use crate::{
    db::{self, image::ImagePool, main::area::schema::Area},
    Error, Result,
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
    pub area_id: String,
    pub image_base64: String,
    #[serde(default)]
    pub image_type: Option<String>,
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

pub async fn run(params: Params, pool: &Pool, image_pool: &ImagePool) -> Result<Res> {
    let area = db::main::area::queries::select_by_id_or_alias(&params.area_id, pool).await?;
    let image_type = params.image_type.as_deref().unwrap_or("square");
    let bytes = BASE64_STANDARD.decode(params.image_base64)?;
    let ext = super::generate_area_icons::detect_ext(&bytes)
        .ok_or(Error::Other("unsupported image format".into()))?;
    let file_name = format!("{}_{}.{}", area.id, image_type, ext);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(format!(
            "/srv/http/static.btcmap.org/images/areas/{file_name}"
        ))?;
    file.write_all(&bytes)?;
    file.flush()?;

    let dims = actix_web::web::block({
        let bytes = bytes.clone();
        move || super::generate_area_icons::decode_dimensions(&bytes)
    })
    .await?;

    if let Some((width, height)) = dims {
        let bytes_len = bytes.len() as i64;
        let existing =
            db::image::area::queries::select_by_area_id_and_type(area.id, image_type, image_pool)
                .await?;

        if let Some(existing) = &existing {
            if existing.image_data == bytes {
                // already in sync, skip DB write
            } else {
                db::image::area::queries::delete(existing.id, image_pool).await?;
                db::image::area::queries::insert(
                    area.id,
                    image_type,
                    bytes,
                    width as i64,
                    height as i64,
                    bytes_len,
                    image_pool,
                )
                .await?;
            }
        } else {
            db::image::area::queries::insert(
                area.id,
                image_type,
                bytes,
                width as i64,
                height as i64,
                bytes_len,
                image_pool,
            )
            .await?;
        }
    }

    let url = format!("https://static.btcmap.org/images/areas/{file_name}");
    let tag_key = format!("icon:{image_type}");
    let patch_set = Map::from_iter([(tag_key, url.into())]);
    let area = db::main::area::queries::patch_tags(area.id, patch_set, pool).await?;
    Ok(area.into())
}
