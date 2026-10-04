use crate::{
    db::{self, image::ImagePool},
    service, Result,
};
use deadpool_sqlite::Pool;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Serialize)]
pub struct Res {
    pub areas_checked: i64,
    pub images_inserted: i64,
    pub images_identical: i64,
    pub images_failed: i64,
    pub failures: Vec<Failure>,
    pub time_s: f64,
}

#[derive(Serialize)]
pub struct Failure {
    pub area_id: i64,
    pub alias: String,
    pub url: String,
    pub reason: String,
}

pub async fn run(main_pool: &Pool, image_pool: &ImagePool) -> Result<Res> {
    let started_at = OffsetDateTime::now_utc();
    let areas = db::main::area::queries::select_with_icon_square(main_pool).await?;
    let areas_checked = areas.len() as i64;

    let mut images_inserted = 0i64;
    let mut images_identical = 0i64;
    let mut images_failed = 0i64;
    let mut failures = Vec::new();

    let client = reqwest::Client::new();

    for area in areas {
        let Some(url) = area
            .tags
            .get("icon:square")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned())
        else {
            images_failed += 1;
            failures.push(Failure {
                area_id: area.id,
                alias: area.alias(),
                url: String::new(),
                reason: "icon:square tag is not a string".into(),
            });
            continue;
        };

        let response = match client.get(&url).send().await {
            Ok(res) => res,
            Err(e) => {
                images_failed += 1;
                failures.push(Failure {
                    area_id: area.id,
                    alias: area.alias(),
                    url: url.clone(),
                    reason: format!("request failed: {e}"),
                });
                continue;
            }
        };

        if !response.status().is_success() {
            images_failed += 1;
            failures.push(Failure {
                area_id: area.id,
                alias: area.alias(),
                url: url.clone(),
                reason: format!("HTTP {}", response.status()),
            });
            continue;
        }

        let bytes = match response.bytes().await {
            Ok(b) => b.to_vec(),
            Err(e) => {
                images_failed += 1;
                failures.push(Failure {
                    area_id: area.id,
                    alias: area.alias(),
                    url: url.clone(),
                    reason: format!("failed to read body: {e}"),
                });
                continue;
            }
        };

        let dims = match actix_web::web::block({
            let bytes = bytes.clone();
            move || service::image::decode_dimensions(&bytes)
        })
        .await
        {
            Ok(Some(dims)) => dims,
            Ok(None) => {
                images_failed += 1;
                failures.push(Failure {
                    area_id: area.id,
                    alias: area.alias(),
                    url: url.clone(),
                    reason: "failed to decode image dimensions".into(),
                });
                continue;
            }
            Err(e) => {
                images_failed += 1;
                failures.push(Failure {
                    area_id: area.id,
                    alias: area.alias(),
                    url: url.clone(),
                    reason: format!("failed to decode image dimensions: {e}"),
                });
                continue;
            }
        };

        let (width, height) = (dims.0 as i64, dims.1 as i64);

        let existing =
            db::image::area::queries::select_by_area_id_and_type(area.id, "square", image_pool)
                .await?;

        let bytes_len = bytes.len() as i64;

        if let Some(existing) = &existing {
            if existing.image_data == bytes {
                images_identical += 1;
                continue;
            }
            db::image::area::queries::delete(existing.id, image_pool).await?;
        }

        db::image::area::queries::insert(
            area.id, "square", bytes, width, height, bytes_len, image_pool,
        )
        .await?;
        images_inserted += 1;
    }

    Ok(Res {
        areas_checked,
        images_inserted,
        images_identical,
        images_failed,
        failures,
        time_s: (OffsetDateTime::now_utc() - started_at).as_seconds_f64(),
    })
}
