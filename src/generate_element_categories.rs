use crate::db;
use crate::model::Element;
use crate::Connection;
use rusqlite::named_params;
use serde_json::Value;

pub async fn generate_element_categories(db_conn: Connection) {
    log::info!("Generating element categories");

    let elements: Vec<Element> = db_conn
        .prepare(db::ELEMENT_SELECT_ALL)
        .unwrap()
        .query_map([], db::mapper_element_full())
        .unwrap()
        .filter(|it| it.is_ok())
        .map(|it| it.unwrap())
        .collect();

    log::info!("Found {} elements", elements.len());

    let mut known = 0;
    let mut unknown = 0;

    for element in &elements {
        let new_category = element.category();
        let old_category = element.tags["category"].as_str().unwrap_or("");

        if new_category != old_category {
            log::info!(
                "Updating category for element {} ({old_category} -> {new_category})",
                &element.id,
            );

            db_conn
                .execute(
                    db::ELEMENT_INSERT_TAG,
                    named_params! {
                        ":element_id": &element.id,
                        ":tag_name": "$.category",
                        ":tag_value": &new_category,
                    },
                )
                .unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }

        if new_category == "other" {
            unknown += 1;
        } else {
            known += 1;
        }
    }

    log::info!(
        "Finished generating categories. Known: {known}, unknown: {unknown}, coverage: {:.2}%",
        known as f64 / (known as f64 + unknown as f64) * 100.0
    );
}

impl Element {
    pub fn category(&self) -> String {
        let tags: &Value = &self.osm_json["tags"];

        let amenity = tags["amenity"].as_str().unwrap_or("");

        let mut category: &str = "other";

        if amenity == "atm" {
            category = "atm";
        }

        category.to_string()
    }
}
