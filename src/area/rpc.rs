use super::Area;
use crate::element::{self, Element};
use crate::event::Event;
use crate::Result;
use crate::{area, auth::Token, discord, Error};
use deadpool_sqlite::Pool;
use jsonrpc_v2::{Data, Params};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::info;

#[derive(Deserialize)]
pub struct GetTrendingAreasArgs {
    pub token: String,
    pub period_start: String,
    pub period_end: String,
}

#[derive(Serialize)]
pub struct TrendingArea {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub events: i64,
    pub created: i64,
    pub updated: i64,
    pub deleted: i64,
}

pub async fn get_trending_countries(
    Params(args): Params<GetTrendingAreasArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Vec<TrendingArea>> {
    pool.get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let period_start =
        OffsetDateTime::parse(&format!("{}T00:00:00Z", args.period_start), &Rfc3339).unwrap();
    let period_end =
        OffsetDateTime::parse(&format!("{}T00:00:00Z", args.period_end), &Rfc3339).unwrap();
    pool.get()
        .await?
        .interact(move |conn| get_trending_areas("country", &period_start, &period_end, conn))
        .await?
}

pub async fn get_trending_communities(
    Params(args): Params<GetTrendingAreasArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Vec<TrendingArea>> {
    pool.get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let period_start =
        OffsetDateTime::parse(&format!("{}T00:00:00Z", args.period_start), &Rfc3339).unwrap();
    let period_end =
        OffsetDateTime::parse(&format!("{}T00:00:00Z", args.period_end), &Rfc3339).unwrap();
    pool.get()
        .await?
        .interact(move |conn| get_trending_areas("community", &period_start, &period_end, conn))
        .await?
}

fn get_trending_areas(
    r#type: &str,
    period_start: &OffsetDateTime,
    period_end: &OffsetDateTime,
    conn: &Connection,
) -> Result<Vec<TrendingArea>> {
    let events = Event::select_created_between(&period_start, &period_end, &conn)?;
    let areas: Vec<Area> = Area::select_all(None, &conn)?
        .into_iter()
        .filter(|it| it.deleted_at == None)
        .collect();
    let mut areas_to_events: HashMap<i64, Vec<&Event>> = HashMap::new();
    for area in &areas {
        areas_to_events.insert(area.id, vec![]);
    }
    for event in &events {
        let element = Element::select_by_id(event.element_id, &conn)?.unwrap();
        let element_area_ids: Vec<i64> = if element.deleted_at.is_none() {
            element
                .tag("areas")
                .as_array()
                .unwrap()
                .iter()
                .map(|it| it["id"].as_i64().unwrap())
                .collect()
        } else {
            element::service::find_areas(&element, &areas)?
                .iter()
                .map(|it| it.id)
                .collect()
        };
        for element_area_id in element_area_ids {
            if !areas_to_events.contains_key(&element_area_id) {
                areas_to_events.insert(element_area_id, vec![]);
            }
            let area_events = areas_to_events.get_mut(&element_area_id).unwrap();
            area_events.push(event);
        }
    }
    let mut trending_areas: Vec<_> = areas_to_events
        .into_iter()
        .map(|it| (Area::select_by_id(it.0, &conn).unwrap().unwrap(), it.1))
        .collect();
    trending_areas.sort_by(|x, y| y.1.len().cmp(&x.1.len()));
    Ok(trending_areas
        .into_iter()
        .filter(|it| it.0.tags.contains_key("type") && it.0.tags["type"].as_str() == Some(r#type))
        .map(|it| {
            let mut created: Vec<&Event> = vec![];
            let mut updated: Vec<&Event> = vec![];
            let mut deleted: Vec<&Event> = vec![];
            for event in &it.1 {
                match event.r#type.as_str() {
                    "create" => created.push(&event),
                    "update" => updated.push(&event),
                    "delete" => deleted.push(&event),
                    _ => {}
                }
            }
            TrendingArea {
                id: it.0.id,
                name: it.0.name(),
                url: format!("https://btcmap.org/{}/{}", r#type, it.0.alias()),
                events: it.1.len() as i64,
                created: created.len() as i64,
                updated: updated.len() as i64,
                deleted: deleted.len() as i64,
            }
        })
        .collect())
}

#[derive(Deserialize)]
pub struct RemoveArgs {
    pub token: String,
    pub id: String,
}

pub async fn remove(
    Params(args): Params<RemoveArgs>,
    pool: Data<Arc<Pool>>,
) -> Result<Area, Error> {
    let token = pool
        .get()
        .await?
        .interact(move |conn| Token::select_by_secret(&args.token, conn))
        .await??
        .unwrap();
    let area = pool
        .get()
        .await?
        .interact(move |conn| area::service::soft_delete(&args.id, conn))
        .await??;
    let log_message = format!(
        "{} removed area {} https://api.btcmap.org/v3/areas/{}",
        token.owner,
        area.name(),
        area.id,
    );
    info!(log_message);
    discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
    Ok(area)
}
