use crate::{db, Result};
use deadpool_sqlite::Pool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    pub query: String,
}

#[derive(Serialize)]
pub struct Res {
    pub name: String,
    pub r#type: String,
    pub id: i64,
}

pub async fn run(params: Params, pool: &Pool) -> Result<Vec<Res>> {
    let areas = db::area::queries::select_by_search_query(&params.query, pool).await?;
    let mut res_areas: Vec<Res> = areas
        .into_iter()
        .map(|it| Res {
            name: it.name(),
            r#type: "area".into(),
            id: it.id,
        })
        .collect();
    let elements = db::element::queries::select_by_search_query(&params.query, pool).await?;
    let mut res_elements: Vec<Res> = elements
        .into_iter()
        .map(|it| Res {
            name: it.name(),
            r#type: "element".into(),
            id: it.id,
        })
        .collect();
    let events = db::event::queries::select_all(pool).await?;
    let events: Vec<_> = events
        .into_iter()
        .filter(|it| {
            it.name
                .to_uppercase()
                .contains(&params.query.to_uppercase())
        })
        .collect();
    let mut res_events: Vec<Res> = events
        .into_iter()
        .map(|it| Res {
            name: it.name,
            r#type: "event".into(),
            id: it.id,
        })
        .collect();
    let mut res = vec![];
    res.append(&mut res_areas);
    res.append(&mut res_elements);
    res.append(&mut res_events);
    Ok(res)
}
