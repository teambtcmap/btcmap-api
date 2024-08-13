use crate::area::Area;
use crate::element::{self, Element};
use crate::event::Event;
use crate::{command::db::open_connection, Result};
use std::collections::HashMap;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::{info, warn};

// Example: btcmap-api trending-areas 2024-07-01 2024-08-01
pub fn run(args: Vec<String>) -> Result<()> {
    let period_start = args.get(2).unwrap();
    info!(period_start);
    let period_start =
        OffsetDateTime::parse(&format!("{period_start}T00:00:00Z"), &Rfc3339).unwrap();
    let period_end = args.get(3).unwrap();
    info!(period_end);
    let period_end = OffsetDateTime::parse(&format!("{period_end}T00:00:00Z"), &Rfc3339).unwrap();
    let conn = open_connection()?;
    let events = Event::select_created_between(&period_start, &period_end, &conn)?;
    info!(
        count = events.len(),
        "Found {} events between {} and {}",
        events.len(),
        period_start.format(&Rfc3339)?,
        period_end.format(&Rfc3339)?
    );
    let areas: Vec<Area> = Area::select_all(None, &conn)?
        .into_iter()
        .filter(|it| it.deleted_at == None)
        .collect();
    let mut areas_to_events: HashMap<i64, Vec<i64>> = HashMap::new();

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
                warn!(element_area_id, "Element contains deleted area");
                areas_to_events.insert(element_area_id, vec![]);
            }

            let area_events = areas_to_events.get_mut(&element_area_id).unwrap();
            area_events.push(event.id);
        }
    }

    let mut trending_areas: Vec<_> = areas_to_events
        .into_iter()
        .map(|it| (Area::select_by_id(it.0, &conn).unwrap().unwrap(), it.1))
        .collect();
    trending_areas.sort_by(|x, y| y.1.len().cmp(&x.1.len()));

    let trending_countries: Vec<_> = trending_areas
        .iter()
        .filter(|it| it.0.tags["type"].as_str().unwrap() == "country")
        .collect();
    let trending_communities: Vec<_> = trending_areas
        .iter()
        .filter(|it| it.0.tags["type"].as_str().unwrap() == "community")
        .collect();

    for (i, area) in trending_countries.iter().enumerate() {
        println!("{} {} {}", i + 1, area.0.name(), area.1.len());
    }

    for (i, area) in trending_communities.iter().enumerate() {
        println!("{} {} {} {}", i + 1, area.0.id, area.0.name(), area.1.len());
    }

    Ok(())
}
