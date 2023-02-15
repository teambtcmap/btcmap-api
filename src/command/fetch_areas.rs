use std::collections::BTreeMap;

use crate::model::area;
use crate::Error;
use crate::Result;
use reqwest::StatusCode;
use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde_json::Map;
use serde_json::Value;

#[derive(Deserialize)]
struct ImportedArea {
    pub id: String,
    pub tags: Map<String, Value>,
}

pub async fn run(db: Connection, url: String) -> Result<()> {
    log::info!("Fetching areas");
    log::info!("Querying {url}");

    let res = reqwest::get(url).await?;

    if res.status() != StatusCode::OK {
        Err(Error::Other(format!(
            "Unexpected status code: {:?}",
            res.status(),
        )))?;
    }

    let body = res.text().await?;
    let new_areas: Vec<ImportedArea> = serde_json::from_str(&body)?;

    for new_area in &new_areas {
        let old_area = db
            .query_row(
                area::SELECT_BY_ID,
                named_params! { ":id": new_area.id },
                area::SELECT_BY_ID_MAPPER,
            )
            .optional()?;

        match old_area {
            Some(old_area) => {
                log::info!("Area {} already exists", new_area.id);

                let mut old_tags_sorted: BTreeMap<String, Value> = BTreeMap::new();
                let mut new_tags_sorted: BTreeMap<String, Value> = BTreeMap::new();

                for (k, v) in &old_area.tags {
                    old_tags_sorted.insert(k.clone(), v.clone());
                }

                for (k, v) in &new_area.tags {
                    new_tags_sorted.insert(k.clone(), v.clone());
                }

                let old_tags_str = serde_json::to_string_pretty(&old_tags_sorted).unwrap();
                let new_tags_str = serde_json::to_string_pretty(&new_tags_sorted).unwrap();

                if old_tags_str == new_tags_str {
                    log::info!("Tags are identical, skipping {}", new_area.id);
                    continue;
                } else {
                    let diff = diff::lines(&old_tags_str, &new_tags_str);

                    for line in diff {
                        match line {
                            diff::Result::Left(v) => {
                                log::info!("- {}", v);
                            }
                            diff::Result::Right(v) => {
                                log::info!("+ {}", v);
                            }
                            diff::Result::Both(..) => {}
                        }
                    }
                }

                let mut merged_tags = BTreeMap::new();
                merged_tags.append(&mut old_tags_sorted);
                merged_tags.append(&mut new_tags_sorted);

                db.execute(
                    area::INSERT_TAGS,
                    named_params! {
                        ":area_id": new_area.id,
                        ":tags": serde_json::to_string(&merged_tags).unwrap(),
                    },
                )?;
            }
            None => {
                log::info!("Area {} doesn't exist", new_area.id);

                let mut new_tags_sorted: BTreeMap<String, Value> = BTreeMap::new();

                for (k, v) in &new_area.tags {
                    new_tags_sorted.insert(k.clone(), v.clone());
                }

                db.execute(
                    area::INSERT,
                    named_params! {
                        ":id": new_area.id,
                    },
                )?;

                db.execute(
                    area::INSERT_TAGS,
                    named_params! {
                        ":area_id": new_area.id,
                        ":tags": serde_json::to_string(&new_tags_sorted).unwrap(),
                    },
                )?;
            }
        }
    }

    log::info!("Fetched {} areas", new_areas.len());
    log::info!("Finished fetching  areas");

    Ok(())
}
