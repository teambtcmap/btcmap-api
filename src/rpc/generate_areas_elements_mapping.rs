use crate::{
    admin::Admin,
    area_element::{self, service::Diff},
    conf::Conf,
    discord,
    element::Element,
    Result,
};
use deadpool_sqlite::Pool;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Params {
    from_element_id: i64,
    to_element_id: i64,
}

#[derive(Serialize)]
pub struct Res {
    pub affected_elements: Vec<Diff>,
}

pub async fn run(params: Params, admin: &Admin, pool: &Pool, conf: &Conf) -> Result<Res> {
    let res = pool
        .get()
        .await?
        .interact(move |conn| {
            generate_areas_elements_mapping(params.from_element_id, params.to_element_id, conn)
        })
        .await??;
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} generated areas to elements mappings (id range {}..{}, elements affected: {})",
            admin.name,
            params.from_element_id,
            params.to_element_id,
            res.affected_elements.len()
        )
    ).await;
    Ok(res)
}

fn generate_areas_elements_mapping(
    from_element_id: i64,
    to_element_id: i64,
    conn: &mut Connection,
) -> Result<Res> {
    let mut elements: Vec<Element> = vec![];
    for element_id in from_element_id..=to_element_id {
        let Some(element) = Element::select_by_id(element_id, conn)? else {
            continue;
        };
        elements.push(element);
    }
    let affected_elements = area_element::service::generate_mapping(&elements, conn)?;
    Ok(Res { affected_elements })
}
