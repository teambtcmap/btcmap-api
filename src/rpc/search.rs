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
    let areas = db::area::queries_async::select_by_search_query(&params.query, pool).await?;
    let mut res_areas: Vec<Res> = areas
        .into_iter()
        .map(|it| Res {
            name: it.name(),
            r#type: "area".into(),
            id: it.id,
        })
        .collect();
    let elements = db::element::queries_async::select_by_search_query(params.query, pool).await?;
    let mut res_elements: Vec<Res> = elements
        .into_iter()
        .map(|it| Res {
            name: it.name(),
            r#type: "element".into(),
            id: it.id,
        })
        .collect();
    let mut res = vec![];
    res.append(&mut res_areas);
    res.append(&mut res_elements);
    Ok(res)
}
