use crate::{admin, area::Area, element::Element, element_comment::ElementComment, Result};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const NAME: &str = "get_most_commented_countries";

#[derive(Deserialize)]
pub struct Args {
    pub password: String,
    pub period_start: String,
    pub period_end: String,
}

#[derive(Serialize)]
pub struct Res {
    id: i64,
    name: String,
    comments: i64,
}

pub async fn run(Params(args): Params<Args>, pool: Data<Arc<Pool>>) -> Result<Vec<Res>> {
    admin::service::check_rpc(&args.password, NAME, &pool).await?;
    let period_start =
        OffsetDateTime::parse(&format!("{}T00:00:00Z", args.period_start), &Rfc3339)?;
    let period_end = OffsetDateTime::parse(&format!("{}T00:00:00Z", args.period_end), &Rfc3339)?;
    pool.get()
        .await?
        .interact(move |conn| get_most_commented_countries(&period_start, &period_end, conn))
        .await?
}

fn get_most_commented_countries(
    period_start: &OffsetDateTime,
    period_end: &OffsetDateTime,
    conn: &Connection,
) -> Result<Vec<Res>> {
    let comments = ElementComment::select_updated_since(period_start, None, conn)?;
    let comments: Vec<ElementComment> = comments
        .into_iter()
        .filter(|it| it.created_at < *period_end)
        .collect();
    let mut areas_to_comments: HashMap<i64, Vec<&ElementComment>> = HashMap::new();
    for comment in &comments {
        let element = Element::select_by_id(comment.element_id, conn)?.ok_or(format!(
            "There is no element with id = {}",
            comment.element_id
        ))?;
        if element.tags.contains_key("areas") {
            let areas = element.tag("areas").as_array().ok_or("Not an array")?;
            for area in areas {
                let area_id = area["id"].as_i64().ok_or("Not an integer")?;
                if !areas_to_comments.contains_key(&area_id) {
                    areas_to_comments.insert(area_id, vec![]);
                }
                let area_comments = areas_to_comments
                    .get_mut(&area_id)
                    .ok_or("Area key is missing")?;
                area_comments.push(comment);
            }
        }
    }
    let areas_to_comments: Vec<(Area, Vec<&ElementComment>)> = areas_to_comments
        .into_iter()
        .map(|(k, v)| (Area::select_by_id(k, conn).unwrap().unwrap(), v)) // TODO remove unwraps
        .collect();
    let mut res: Vec<Res> = areas_to_comments
        .iter()
        .filter(|it| {
            it.0.tags.contains_key("type") && it.0.tags["type"].as_str() == Some("country")
        })
        .map(|it| Res {
            id: it.0.id,
            name: it.0.name(),
            comments: it.1.len() as i64,
        })
        .collect();
    res.sort_by(|x, y| y.comments.cmp(&x.comments));
    Ok(res)
}
