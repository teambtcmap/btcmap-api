use crate::model::element;
use crate::model::Element;
use crate::Result;
use rusqlite::named_params;
use rusqlite::Connection;
use serde_json::Value;
use tokio::time::sleep;
use tokio::time::Duration;

pub async fn run(db: Connection) -> Result<()> {
    log::info!("Migrating Pouch tags");

    let elements: Vec<Element> = db
        .prepare(element::SELECT_ALL)?
        .query_map([], element::SELECT_ALL_MAPPER)?
        .collect::<Result<_, _>>()?;

    for element in elements {
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
            )?;
            sleep(Duration::from_millis(50)).await;
        }
    }

    log::info!("Finished migrating Pouch tags");

    Ok(())
}
