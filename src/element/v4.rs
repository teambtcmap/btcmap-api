use crate::element::Element;
use crate::log::RequestExtension;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::HttpMessage;
use actix_web::HttpRequest;
use actix_web_lab::extract::Query;
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::info;

#[derive(Deserialize)]
pub struct GetListArgs {
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    updated_since: Option<OffsetDateTime>,
    limit: Option<i64>,
    include_deleted: Option<bool>,
    include_tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct GetSingleArgs {
    include_tags: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct GetItem {
    pub id: i64,
    pub lat: f64,
    pub lon: f64,
    pub tags: Map<String, Value>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

#[get("")]
pub async fn get(
    req: HttpRequest,
    args: Query<GetListArgs>,
    pool: Data<Pool>,
) -> Result<Json<Vec<GetItem>>, Error> {
    let include_tags = args.include_tags.clone().unwrap_or(vec![]);
    info!(tags = serde_json::to_string(&include_tags)?);
    let elements = pool
        .get()
        .await?
        .interact(move |conn| {
            Element::select_updated_since(
                &args
                    .updated_since
                    .unwrap_or(OffsetDateTime::parse("2020-01-01T00:00:00Z", &Rfc3339).unwrap()),
                Some(args.limit.unwrap_or(i64::MAX)),
                args.include_deleted.unwrap_or(true),
                conn,
            )
        })
        .await??;
    req.extensions_mut()
        .insert(RequestExtension::new(elements.len()));
    let items: Vec<GetItem> = elements
        .into_iter()
        .map(|it| GetItem {
            id: it.id,
            lat: it.overpass_data.coord().y,
            lon: it.overpass_data.coord().x,
            tags: generate_tags(&it, &include_tags),
            updated_at: it.updated_at,
            deleted_at: it.deleted_at,
        })
        .collect();
    Ok(Json(items))
}

#[get("{id}")]
pub async fn get_by_id(
    id: Path<String>,
    args: Query<GetSingleArgs>,
    pool: Data<Pool>,
) -> Result<Json<GetItem>, Error> {
    let include_tags = args.include_tags.clone().unwrap_or(vec![]);
    let id_clone = id.clone();
    pool.get()
        .await?
        .interact(move |conn| Element::select_by_id_or_osm_id(&id_clone, conn))
        .await??
        .ok_or(Error::NotFound(format!(
            "Element with id {id} doesn't exist"
        )))
        .map(|it| GetItem {
            id: it.id,
            lat: it.overpass_data.coord().y,
            lon: it.overpass_data.coord().x,
            tags: generate_tags(&it, &include_tags),
            updated_at: it.updated_at,
            deleted_at: it.deleted_at,
        })
        .map(|it| Json(it))
}

pub fn generate_tags(element: &Element, include_tags: &Vec<String>) -> Map<String, Value> {
    let mut res = Map::new();
    let whitelisted_tags = vec![
        "btcmap:icon",
        "btcmap:boost:expires",
        "osm:type",
        "osm:id",
        "name",
        "phone",
        "website",
        "check_date",
        "survey:date",
        "check_date:currency:XBT",
        "addr:street",
        "addr:housenumber",
        "contact:website",
        "opening_hours",
        "contact:phone",
        "contact:email",
        "contact:twitter",
        "contact:instagram",
        "contact:facebook",
        "contact:line",
    ];
    let include_tags: Vec<&String> = include_tags
        .into_iter()
        .filter(|it| whitelisted_tags.contains(&it.as_str()))
        .collect();
    if let Some(osm_tags) = &element.overpass_data.tags {
        for tag in &include_tags {
            if tag.starts_with("btcmap:") || tag.starts_with("osm:") {
                continue;
            }
            if osm_tags.contains_key(tag.as_str()) {
                res.insert(tag.to_string(), osm_tags[tag.as_str()].clone());
            }
        }
    }
    if element.tags.contains_key("icon:android")
        && include_tags.contains(&&"btcmap:icon".to_string())
    {
        res.insert("btcmap:icon".into(), element.tags["icon:android"].clone());
    }
    if element.tags.contains_key("boost:expires")
        && include_tags.contains(&&"btcmap:boost:expires".to_string())
    {
        res.insert(
            "btcmap:boost:expires".into(),
            element.tags["boost:expires"].clone(),
        );
    }
    if include_tags.contains(&&"osm:type".to_string()) {
        res.insert(
            "osm:type".into(),
            Value::String(element.overpass_data.r#type.clone()),
        );
    }
    if include_tags.contains(&&"osm:id".to_string()) {
        res.insert(
            "osm:id".into(),
            Value::Number(element.overpass_data.id.into()),
        );
    }
    res
}

#[cfg(test)]
mod test {
    use crate::element::Element;
    use crate::osm::overpass::OverpassElement;
    use crate::test::mock_db;
    use crate::Result;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use time::macros::datetime;

    #[test]
    async fn get_empty_array() -> Result<()> {
        let db = mock_db().await;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=1")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.len(), 0);
        Ok(())
    }

    #[test]
    async fn get_not_empty_array() -> Result<()> {
        let db = mock_db().await;
        let _element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=1")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        Ok(())
    }

    #[test]
    async fn get_with_limit() -> Result<()> {
        let db = mock_db().await;
        let _element_1 = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        let _element_2 = Element::insert(&OverpassElement::mock(2), &db.conn)?;
        let _element_3 = Element::insert(&OverpassElement::mock(3), &db.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2020-01-01T00:00:00Z&limit=2")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, res.len());
        Ok(())
    }

    #[test]
    async fn get_updated_since() -> Result<()> {
        let db = mock_db().await;
        let element_1 = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        Element::_set_updated_at(element_1.id, &datetime!(2022-01-05 00:00 UTC), &db.conn)?;
        let element_2 = Element::insert(&OverpassElement::mock(2), &db.conn)?;
        let _element_2 =
            Element::_set_updated_at(element_2.id, &datetime!(2022-02-05 00:00 UTC), &db.conn)?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get)),
        )
        .await;
        let req = TestRequest::get()
            .uri("/?updated_since=2022-01-10T00:00:00Z&limit=100")
            .to_request();
        let res: Vec<super::GetItem> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        Ok(())
    }
}
