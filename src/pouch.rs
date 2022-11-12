use crate::model::element;
use crate::model::Element;
use rusqlite::named_params;
use rusqlite::Connection;
use serde_json::Value;
use tokio::time::sleep;
use tokio::time::Duration;

pub async fn pouch(db: Connection) {
    log::info!("Migrating Pouch tags");

    let elements: Vec<Element> = db
        .prepare(element::SELECT_ALL)
        .unwrap()
        .query_map([], element::SELECT_ALL_MAPPER)
        .unwrap()
        .filter(|it| it.is_ok())
        .map(|it| it.unwrap())
        .collect();

    for element in &elements {
        let tags: &Value = &element.osm_json["tags"];
        let tag = tags["payment:pouch"].as_str().unwrap_or("");

        if tag.len() > 0 {
            log::info!("Updating tag for element {}", element.id);

            db.execute(
                element::INSERT_TAG,
                named_params! {
                    ":element_id": element.id,
                    ":tag_name": "$.payment:pouch",
                    ":tag_value": tag,
                },
            )
            .unwrap();
            sleep(Duration::from_millis(50)).await;
        }
    }

    log::info!("Finished migrating Pouch tags");
}
