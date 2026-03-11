use crate::{
    db,
    service::{self, ppq::MINIMAX_M2_5},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

#[derive(Deserialize)]
pub struct Params {
    max_places: i64,
}

#[derive(Serialize)]
pub struct ElementResult {
    pub id: i64,
    pub name: String,
    pub osm_url: String,
    pub original_hours: String,
    pub humanized_hours: String,
}

#[derive(Serialize)]
pub struct Res {
    pub processed: i64,
    pub elements: Vec<ElementResult>,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Res> {
    let elements = db::main::element::queries::select_with_opening_hours_without_humanization(
        params.max_places,
        pool,
    )
    .await?;

    let mut results = Vec::new();

    for element in elements {
        let opening_hours = element.overpass_data.tag("opening_hours");
        if opening_hours.is_empty() {
            continue;
        }
        info!(opening_hours);
        let prompt = format!(
            r#"Translate the following OpenStreetMap opening hours format to a human-readable format.

Source format (OSM opening_hours specification - formal and hard to read):
{}

Target format: Human readable, multi-line string where each day of the week is on a separate line, using full day names in English. Use 24-hour time format.

Example:

Input:
Mo-Fr 09:00-15:00

Output: 
Monday: 9:00-15:00
Tuesday: 9:00-15:00
Wednesday: 9:00-15:00
Thursday: 9:00-15:00
Friday: 9:00-15:00

Input: {}
Output:"#,
            opening_hours, opening_hours
        );

        let human_readable = service::ppq::chat(prompt, MINIMAX_M2_5, pool).await?;

        db::main::element::queries::set_tag(
            element.id,
            "opening_hours:en:human_readable",
            &json!(human_readable),
            pool,
        )
        .await?;

        results.push(ElementResult {
            id: element.id,
            name: element.name(None),
            osm_url: element.osm_url(),
            original_hours: opening_hours.to_string(),
            humanized_hours: human_readable,
        });
    }

    Ok(Res {
        processed: results.len() as i64,
        elements: results,
    })
}
