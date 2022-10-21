use crate::auth::is_from_admin;
use crate::db;
use crate::model::ApiError;
use crate::model::Area;
use crate::model::Element;
use actix_web::get;
use actix_web::post;
use actix_web::web::Data;
use actix_web::web::Form;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::HttpRequest;
use rusqlite::named_params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::sync::Mutex;
use time::Duration;
use time::OffsetDateTime;

use std::ops::Sub;

#[derive(Serialize)]
pub struct GetAreasItem {
    pub id: String,
    pub name: String,
    pub area_type: String,
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
    pub elements: usize,
    pub up_to_date_elements: usize,
}

#[derive(Deserialize)]
struct PostTagsArgsV2 {
    name: String,
    value: String,
}

#[derive(Deserialize)]
pub struct GetAreasArgsV2 {
    updated_since: Option<String>,
}

#[derive(Serialize)]
pub struct GetAreasItemV2 {
    pub id: String,
    pub tags: Value,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
}

impl Into<GetAreasItemV2> for Area {
    fn into(self) -> GetAreasItemV2 {
        GetAreasItemV2 {
            id: self.id,
            tags: self.tags,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

#[get("/areas")]
async fn get(conn: Data<Mutex<Connection>>) -> Result<Json<Vec<GetAreasItem>>, ApiError> {
    let conn = conn.lock()?;

    let areas: Vec<Area> = conn
        .prepare(db::AREA_SELECT_ALL)?
        .query_map([], db::mapper_area_full())?
        .filter(|it| it.is_ok())
        .map(|it| it.unwrap())
        .collect();

    let elements: Vec<Element> = conn
        .prepare(db::ELEMENT_SELECT_ALL)?
        .query_map([], db::mapper_element_full())?
        .filter(|it| it.is_ok())
        .map(|it| it.unwrap())
        .collect();

    let mut res: Vec<GetAreasItem> = vec![];
    let today = OffsetDateTime::now_utc().date();
    let year_ago = today.sub(Duration::days(365));

    for area in areas {
        let area_elements: Vec<&Element> = elements
            .iter()
            .filter(|it| it.data["type"].as_str().unwrap() == "node")
            .filter(|it| {
                let lat = it.data["lat"].as_f64().unwrap();
                let lon = it.data["lon"].as_f64().unwrap();
                lon > area.min_lon && lon < area.max_lon && lat > area.min_lat && lat < area.max_lat
            })
            .collect();

        let elements_len = area_elements.len();

        let up_to_date_elements: Vec<&Element> = area_elements
            .into_iter()
            .filter(|it| {
                (it.data["tags"].get("survey:date").is_some()
                    && it.data["tags"]["survey:date"].as_str().unwrap().to_string()
                        > year_ago.to_string())
                    || (it.data["tags"].get("check_date").is_some()
                        && it.data["tags"]["check_date"].as_str().unwrap().to_string()
                            > year_ago.to_string())
            })
            .collect();

        res.push(GetAreasItem {
            id: area.id,
            name: area.name,
            area_type: area.area_type,
            min_lon: area.min_lon,
            min_lat: area.min_lat,
            max_lon: area.max_lon,
            max_lat: area.max_lat,
            elements: elements_len,
            up_to_date_elements: up_to_date_elements.len(),
        });
    }

    Ok(Json(res))
}

#[get("/v2/areas")]
async fn get_v2(
    args: Query<GetAreasArgsV2>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Vec<GetAreasItemV2>>, ApiError> {
    Ok(Json(match &args.updated_since {
        Some(updated_since) => conn
            .lock()?
            .prepare(db::AREA_SELECT_UPDATED_SINCE)?
            .query_map([updated_since], db::mapper_area_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
        None => conn
            .lock()?
            .prepare(db::AREA_SELECT_ALL)?
            .query_map([], db::mapper_area_full())?
            .filter(|it| it.is_ok())
            .map(|it| it.unwrap().into())
            .collect(),
    }))
}

#[get("/areas/{id}")]
async fn get_by_id(
    path: Path<String>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<GetAreasItem>, ApiError> {
    let id_or_name = path.into_inner();
    let conn = conn.lock()?;

    let area_by_id = conn
        .query_row(db::AREA_SELECT_BY_ID, [&id_or_name], db::mapper_area_full())
        .optional()?;

    match area_by_id {
        Some(area) => area_to_areas_item(area, &conn),
        None => {
            let area_by_name = conn
                .query_row(
                    db::AREA_SELECT_BY_NAME,
                    [&id_or_name],
                    db::mapper_area_full(),
                )
                .optional()?;

            match area_by_name {
                Some(area) => area_to_areas_item(area, &conn),
                None => Result::Err(ApiError {
                    message: format!("Area with id or name {} doesn't exist", &id_or_name)
                        .to_string(),
                }),
            }
        }
    }
}

#[get("/v2/areas/{id}")]
async fn get_by_id_v2(
    path: Path<String>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<GetAreasItemV2>, ApiError> {
    let id_or_name = path.into_inner();
    let conn = conn.lock()?;

    let area_by_id: Option<Area> = conn
        .query_row(db::AREA_SELECT_BY_ID, [&id_or_name], db::mapper_area_full())
        .optional()?;

    match area_by_id {
        Some(area) => Ok(Json(area.into())),
        None => {
            let area_by_name = conn
                .query_row(
                    db::AREA_SELECT_BY_NAME,
                    [&id_or_name],
                    db::mapper_area_full(),
                )
                .optional()?;

            match area_by_name {
                Some(area) => Ok(Json(area.into())),
                None => Result::Err(ApiError {
                    message: format!("Area with id or name {} doesn't exist", &id_or_name)
                        .to_string(),
                }),
            }
        }
    }
}

#[post("/v2/areas/{id}")]
async fn post_v2(
    id: Path<String>,
    req: HttpRequest,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Value>, ApiError> {
    if let Err(err) = is_from_admin(&req) {
        return Err(err);
    };

    let conn = conn.lock()?;

    conn.execute(
        db::AREA_INSERT,
        named_params![
            ":id": id.into_inner(),
        ],
    )?;

    Ok(Json("{}".to_string().into()))
}

#[post("/v2/areas/{id}/tags")]
async fn post_tags_v2(
    id: Path<String>,
    req: HttpRequest,
    args: Form<PostTagsArgsV2>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Value>, ApiError> {
    if let Err(err) = is_from_admin(&req) {
        return Err(err);
    };

    let conn = conn.lock()?;

    let area: Option<Area> = conn
        .query_row(
            db::AREA_SELECT_BY_ID,
            [&id.into_inner()],
            db::mapper_area_full(),
        )
        .optional()?;

    match area {
        Some(area) => {
            if args.value.len() > 0 {
                conn.execute(
                    db::AREA_INSERT_TAG,
                    named_params! {
                        ":area_id": area.id,
                        ":tag_name": format!("$.{}", args.name),
                        ":tag_value": args.value,
                    },
                )?;
            } else {
                conn.execute(
                    db::AREA_DELETE_TAG,
                    named_params! {
                        ":area_id": area.id,
                        ":tag_name": format!("$.{}", args.name),
                    },
                )?;
            }

            Ok(Json("{}".to_string().into()))
        }
        None => Err(ApiError::new("Can't find area")),
    }
}

fn area_to_areas_item(area: Area, conn: &Connection) -> Result<Json<GetAreasItem>, ApiError> {
    let all_elements: Vec<Element> = conn
        .prepare(db::ELEMENT_SELECT_ALL)?
        .query_map([], db::mapper_element_full())?
        .map(|row| row.unwrap())
        .collect();

    let area_elements: Vec<&Element> = all_elements
        .iter()
        .filter(|it| {
            it.lon() > area.min_lon
                && it.lon() < area.max_lon
                && it.lat() > area.min_lat
                && it.lat() < area.max_lat
        })
        .collect();

    let elements_len = area_elements.len();
    let today = OffsetDateTime::now_utc().date();
    let year_ago = today.sub(Duration::days(365));

    let up_to_date_elements: Vec<&Element> = area_elements
        .into_iter()
        .filter(|it| {
            (it.data["tags"].get("survey:date").is_some()
                && it.data["tags"]["survey:date"].as_str().unwrap().to_string()
                    > year_ago.to_string())
                || (it.data["tags"].get("check_date").is_some()
                    && it.data["tags"]["check_date"].as_str().unwrap().to_string()
                        > year_ago.to_string())
        })
        .collect();

    Ok(Json(GetAreasItem {
        id: area.id,
        name: area.name,
        area_type: area.area_type,
        min_lon: area.min_lon,
        min_lat: area.min_lat,
        max_lon: area.max_lon,
        max_lat: area.max_lat,
        elements: elements_len,
        up_to_date_elements: up_to_date_elements.len(),
    }))
}

#[get("/areas/{id}/elements")]
async fn get_area_elements(
    path: Path<String>,
    conn: Data<Mutex<Connection>>,
) -> Result<Json<Vec<Element>>, ApiError> {
    let conn = conn.lock()?;

    let area = conn
        .query_row(
            db::AREA_SELECT_BY_ID,
            [path.into_inner()],
            db::mapper_area_full(),
        )
        .optional()
        .unwrap();

    if let None = area {
        return Ok(Json(vec![]));
    }

    let area = area.unwrap();

    let elements: Vec<Element> = conn
        .prepare(db::ELEMENT_SELECT_ALL)?
        .query_map([], db::mapper_element_full())?
        .filter(|it| it.is_ok())
        .map(|it| it.unwrap())
        .filter(|it| {
            let element_type = it.data["type"].as_str().unwrap();

            if element_type != "node" {
                return false;
            }

            let lat = it.data["lat"].as_f64().unwrap();
            let lon = it.data["lon"].as_f64().unwrap();

            lon > area.min_lon && lon < area.max_lon && lat > area.min_lat && lat < area.max_lat
        })
        .collect();

    Ok(Json(elements))
}
