use crate::{
    area_element::{self, service::Diff},
    conf::Conf,
    db::admin::queries::Admin,
    discord,
    element::Element,
    Result,
};
use deadpool_sqlite::Pool;
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
    let res =
        generate_areas_elements_mapping(params.from_element_id, params.to_element_id, pool).await?;
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

async fn generate_areas_elements_mapping(
    from_element_id: i64,
    to_element_id: i64,
    pool: &Pool,
) -> Result<Res> {
    let mut elements: Vec<Element> = vec![];
    for element_id in from_element_id..=to_element_id {
        let Ok(element) = Element::select_by_id_async(element_id, pool).await else {
            continue;
        };
        elements.push(element);
    }
    let affected_elements = area_element::service::generate_mapping(&elements, pool).await?;
    Ok(Res { affected_elements })
}
