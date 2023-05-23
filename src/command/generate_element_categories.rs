use crate::model::element;
use crate::model::Element;
use crate::Connection;
use crate::Result;
use rusqlite::named_params;
use serde_json::Value;
use tracing::info;

pub async fn run(db: Connection) -> Result<()> {
    info!("Generating element categories");

    let elements: Vec<Element> = db
        .prepare(element::SELECT_ALL)?
        .query_map(
            named_params! { ":limit": std::i32::MAX },
            element::SELECT_ALL_MAPPER,
        )?
        .collect::<Result<Vec<Element>, _>>()?
        .into_iter()
        .filter(|it| it.deleted_at.len() == 0)
        .collect();

    info!(elements = elements.len(), "Loaded elements from database");

    let mut known = 0;
    let mut unknown = 0;

    for element in elements {
        let new_category = element.category();

        let old_category = element
            .tags
            .get("category")
            .unwrap_or(&Value::Null)
            .as_str()
            .unwrap_or("");

        if new_category != old_category {
            info!(element.id, old_category, new_category, "Updating category",);

            db.execute(
                element::INSERT_TAG,
                named_params! {
                    ":element_id": element.id,
                    ":tag_name": "$.category",
                    ":tag_value": new_category,
                },
            )?;
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }

        if new_category == "other" {
            unknown += 1;
        } else {
            known += 1;
        }

        let category_plural = element
            .tags
            .get("category:plural")
            .unwrap_or(&Value::Null)
            .as_str()
            .unwrap_or("");

        if category_plural.len() > 0 {
            info!(element.id, category_plural, "Removing category:plural",);

            db.execute(
                "update element set tags = json_remove(tags, '$.category:plural') where id = :element_id;",
                named_params! { ":element_id": element.id },
            )?;
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }
    }

    let coverage = known as f64 / (known as f64 + unknown as f64) * 100.0;

    info!(
        known,
        unknown,
        coverage = format!("{:.2}", coverage),
        "Finished generating categories",
    );

    Ok(())
}

impl Element {
    pub fn category(&self) -> String {
        let tags: &Value = &self.osm_json["tags"];

        let amenity = tags["amenity"].as_str().unwrap_or("");
        let tourism = tags["tourism"].as_str().unwrap_or("");

        let mut category: &str = "other";

        if amenity == "atm" {
            category = "atm";
        }

        if amenity == "cafe" {
            category = "cafe";
        }

        if amenity == "restaurant" {
            category = "restaurant";
        }

        if amenity == "bar" {
            category = "bar";
        }

        if amenity == "pub" {
            category = "pub";
        }

        if tourism == "hotel" {
            category = "hotel";
        }

        category.to_string()
    }
}
