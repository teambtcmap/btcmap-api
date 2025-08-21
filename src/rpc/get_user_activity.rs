use crate::{
    db::{self, element::schema::Element, element_event::schema::ElementEvent},
    Result,
};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    pub id: String,
    pub limit: i64,
}

#[derive(Serialize)]
pub struct Res {
    #[serde(with = "time::serde::rfc3339")]
    pub date: OffsetDateTime,
    pub message: String,
    pub osm_url: String,
    pub btcmap_url: String,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Vec<Res>> {
    let user = db::osm_user::queries_async::select_by_id_or_name(params.id, pool).await?;
    let user_events =
        db::element_event::queries::select_by_user(user.id, params.limit, pool).await?;
    let mut user_events_to_elements: Vec<(ElementEvent, Element)> = vec![];
    for event in user_events {
        let element_id = event.element_id;
        user_events_to_elements.push((
            event,
            db::element::queries::select_by_id(element_id, pool).await?,
        ));
    }
    let res = user_events_to_elements
        .into_iter()
        .map(|it| Res {
            date: it.0.created_at,
            message: format!(
                "{} {}d element {}",
                user.osm_data.display_name,
                it.0.r#type,
                it.1.name(),
            ),
            osm_url: format!(
                "https://www.openstreetmap.org/{}/{}",
                it.1.overpass_data.r#type, it.1.overpass_data.id
            ),
            btcmap_url: format!(
                "https://btcmap.org/merchant/{}:{}",
                it.1.overpass_data.r#type, it.1.overpass_data.id
            ),
        })
        .collect();
    Ok(res)
}
