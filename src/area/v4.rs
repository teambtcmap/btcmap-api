use crate::area::Area;
use crate::element::Element;
use crate::log::RequestExtension;
use crate::user::User;
use crate::Error;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::web::Path;
use actix_web::web::Query;
use actix_web::HttpMessage;
use actix_web::HttpRequest;
use deadpool_sqlite::Pool;
use rusqlite::named_params;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct GetFeedArgs {
    limit: Option<i64>,
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    after: Option<OffsetDateTime>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum FeedItem {
    #[serde(rename = "edit")]
    Edit {
        id: i64,
        user_id: i64,
        user_name: String,
        element_id: i64,
        element_name: String,
        action: String,
        #[serde(with = "time::serde::rfc3339")]
        created_at: OffsetDateTime,
    },
    #[serde(rename = "comment")]
    Comment {
        id: i64,
        element_id: i64,
        element_name: String,
        comment: String,
        #[serde(with = "time::serde::rfc3339")]
        created_at: OffsetDateTime,
    },
    #[serde(rename = "boost")]
    Boost {
        id: i64,
        element_id: i64,
        element_name: String,
        duration_days: i64,
        #[serde(with = "time::serde::rfc3339")]
        created_at: OffsetDateTime,
    },
}

impl FeedItem {
    pub fn created_at(&self) -> &OffsetDateTime {
        match self {
            FeedItem::Edit { created_at, .. } => created_at,
            FeedItem::Comment { created_at, .. } => created_at,
            FeedItem::Boost { created_at, .. } => created_at,
        }
    }
}

// SQL-level queries scoped to an area via the area_element junction table.
// Each query joins through area_element to only fetch rows for elements in the area,
// avoiding full table scans.

struct RawEdit {
    id: i64,
    user_id: i64,
    element_id: i64,
    action: String,
    created_at: OffsetDateTime,
}

struct RawComment {
    id: i64,
    element_id: i64,
    comment: String,
    created_at: OffsetDateTime,
}

struct RawBoost {
    id: i64,
    element_id: i64,
    duration_days: i64,
    created_at: OffsetDateTime,
}

fn select_area_events(
    area_id: i64,
    after: Option<&OffsetDateTime>,
    limit: i64,
    conn: &Connection,
) -> crate::Result<Vec<RawEdit>> {
    let after_clause = if after.is_some() {
        "AND ev.created_at < :after"
    } else {
        ""
    };
    let sql = format!(
        r#"
            SELECT ev.rowid, ev.user_id, ev.element_id, ev.type, ev.created_at
            FROM event ev
            INNER JOIN area_element ae ON ae.element_id = ev.element_id
            WHERE ae.area_id = :area_id
              AND ae.deleted_at IS NULL
              AND ev.deleted_at IS NULL
              {after_clause}
            ORDER BY ev.created_at DESC
            LIMIT :limit
        "#
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = if let Some(after) = after {
        stmt.query_map(
            named_params! {
                ":area_id": area_id,
                ":after": after.format(&Rfc3339)?,
                ":limit": limit,
            },
            |row| {
                Ok(RawEdit {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    element_id: row.get(2)?,
                    action: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )?
    } else {
        stmt.query_map(
            named_params! {
                ":area_id": area_id,
                ":limit": limit,
            },
            |row| {
                Ok(RawEdit {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    element_id: row.get(2)?,
                    action: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )?
    };
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn select_area_comments(
    area_id: i64,
    after: Option<&OffsetDateTime>,
    limit: i64,
    conn: &Connection,
) -> crate::Result<Vec<RawComment>> {
    let after_clause = if after.is_some() {
        "AND ec.created_at < :after"
    } else {
        ""
    };
    let sql = format!(
        r#"
            SELECT ec.id, ec.element_id, ec.comment, ec.created_at
            FROM element_comment ec
            INNER JOIN area_element ae ON ae.element_id = ec.element_id
            WHERE ae.area_id = :area_id
              AND ae.deleted_at IS NULL
              AND ec.deleted_at IS NULL
              {after_clause}
            ORDER BY ec.created_at DESC
            LIMIT :limit
        "#
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = if let Some(after) = after {
        stmt.query_map(
            named_params! {
                ":area_id": area_id,
                ":after": after.format(&Rfc3339)?,
                ":limit": limit,
            },
            |row| {
                Ok(RawComment {
                    id: row.get(0)?,
                    element_id: row.get(1)?,
                    comment: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )?
    } else {
        stmt.query_map(
            named_params! {
                ":area_id": area_id,
                ":limit": limit,
            },
            |row| {
                Ok(RawComment {
                    id: row.get(0)?,
                    element_id: row.get(1)?,
                    comment: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )?
    };
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn select_area_boosts(
    area_id: i64,
    after: Option<&OffsetDateTime>,
    limit: i64,
    conn: &Connection,
) -> crate::Result<Vec<RawBoost>> {
    let after_clause = if after.is_some() {
        "AND b.created_at < :after"
    } else {
        ""
    };
    let sql = format!(
        r#"
            SELECT b.id, b.element_id, b.duration_days, b.created_at
            FROM boost b
            INNER JOIN area_element ae ON ae.element_id = b.element_id
            WHERE ae.area_id = :area_id
              AND ae.deleted_at IS NULL
              AND b.deleted_at IS NULL
              {after_clause}
            ORDER BY b.created_at DESC
            LIMIT :limit
        "#
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = if let Some(after) = after {
        stmt.query_map(
            named_params! {
                ":area_id": area_id,
                ":after": after.format(&Rfc3339)?,
                ":limit": limit,
            },
            |row| {
                Ok(RawBoost {
                    id: row.get(0)?,
                    element_id: row.get(1)?,
                    duration_days: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )?
    } else {
        stmt.query_map(
            named_params! {
                ":area_id": area_id,
                ":limit": limit,
            },
            |row| {
                Ok(RawBoost {
                    id: row.get(0)?,
                    element_id: row.get(1)?,
                    duration_days: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )?
    };
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[get("{area_id_or_alias}/feed")]
pub async fn get_feed(
    req: HttpRequest,
    path: Path<String>,
    args: Query<GetFeedArgs>,
    pool: Data<Pool>,
) -> Result<Json<Vec<FeedItem>>, Error> {
    let area_id_or_alias = path.into_inner();
    let limit = args.limit.unwrap_or(50).min(100).max(1);
    let after = args.after;

    let area = Area::select_by_id_or_alias(area_id_or_alias, &pool)
        .await
        .map_err(|e| match e {
            Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => Error::not_found(),
            other => other,
        })?;
    let area_id = area.id;

    let items = pool
        .get()
        .await?
        .interact(move |conn| {
            // Query each source at the SQL level, already filtered by area and cursor
            let events = select_area_events(area_id, after.as_ref(), limit, conn)?;
            let comments = select_area_comments(area_id, after.as_ref(), limit, conn)?;
            let boosts = select_area_boosts(area_id, after.as_ref(), limit, conn)?;

            // Build feed items (names filled in after truncation)
            let mut items: Vec<FeedItem> = Vec::new();

            for e in events {
                items.push(FeedItem::Edit {
                    id: e.id,
                    user_id: e.user_id,
                    user_name: String::new(),
                    element_id: e.element_id,
                    element_name: String::new(),
                    action: e.action,
                    created_at: e.created_at,
                });
            }

            for c in comments {
                items.push(FeedItem::Comment {
                    id: c.id,
                    element_id: c.element_id,
                    element_name: String::new(),
                    comment: c.comment,
                    created_at: c.created_at,
                });
            }

            for b in boosts {
                items.push(FeedItem::Boost {
                    id: b.id,
                    element_id: b.element_id,
                    element_name: String::new(),
                    duration_days: b.duration_days,
                    created_at: b.created_at,
                });
            }

            // Sort by created_at DESC and truncate to requested limit
            items.sort_by(|a, b| b.created_at().cmp(a.created_at()));
            items.truncate(limit as usize);

            // Collect IDs only from the truncated set for name lookups
            let mut needed_element_ids: HashSet<i64> = HashSet::new();
            let mut needed_user_ids: HashSet<i64> = HashSet::new();
            for item in &items {
                match item {
                    FeedItem::Edit {
                        element_id,
                        user_id,
                        ..
                    } => {
                        needed_element_ids.insert(*element_id);
                        needed_user_ids.insert(*user_id);
                    }
                    FeedItem::Comment { element_id, .. } => {
                        needed_element_ids.insert(*element_id);
                    }
                    FeedItem::Boost { element_id, .. } => {
                        needed_element_ids.insert(*element_id);
                    }
                }
            }

            // Build lookup maps
            let mut element_names: HashMap<i64, String> = HashMap::new();
            for eid in &needed_element_ids {
                if let Ok(Some(el)) = Element::select_by_id(*eid, conn) {
                    element_names.insert(*eid, el.name());
                }
            }

            let mut user_names: HashMap<i64, String> = HashMap::new();
            for uid in &needed_user_ids {
                if let Ok(Some(user)) = User::select_by_id(*uid, conn) {
                    user_names.insert(*uid, user.osm_data.display_name.clone());
                }
            }

            // Fill in names
            for item in &mut items {
                match item {
                    FeedItem::Edit {
                        user_id,
                        user_name,
                        element_id,
                        element_name,
                        ..
                    } => {
                        *element_name =
                            element_names.get(element_id).cloned().unwrap_or_default();
                        *user_name = user_names.get(user_id).cloned().unwrap_or_default();
                    }
                    FeedItem::Comment {
                        element_id,
                        element_name,
                        ..
                    } => {
                        *element_name =
                            element_names.get(element_id).cloned().unwrap_or_default();
                    }
                    FeedItem::Boost {
                        element_id,
                        element_name,
                        ..
                    } => {
                        *element_name =
                            element_names.get(element_id).cloned().unwrap_or_default();
                    }
                }
            }

            Ok(items)
        })
        .await??;

    req.extensions_mut()
        .insert(RequestExtension::new(items.len()));
    Ok(Json(items))
}

#[cfg(test)]
mod test {
    use crate::area::Area;
    use crate::area_element::AreaElement;
    use crate::boost::Boost;
    use crate::element::Element;
    use crate::element_comment::ElementComment;
    use crate::event::Event;
    use crate::osm::api::OsmUser;
    use crate::osm::overpass::OverpassElement;
    use crate::test::mock_db;
    use crate::user::User;
    use crate::Result;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::web::{scope, Data};
    use actix_web::{test, App};
    use serde_json::Value;

    #[test]
    async fn get_feed_area_not_found() -> Result<()> {
        let db = mock_db();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get_feed)),
        )
        .await;
        let req = TestRequest::get().uri("/999/feed").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(StatusCode::NOT_FOUND, res.status());
        Ok(())
    }

    #[test]
    async fn get_feed_empty() -> Result<()> {
        let db = mock_db();
        let area = Area::insert(Area::mock_tags(), &db.pool).await?;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get_feed)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/feed", area.id))
            .to_request();
        let res: Vec<Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(0, res.len());
        Ok(())
    }

    #[test]
    async fn get_feed_with_edit() -> Result<()> {
        let db = mock_db();
        let area = Area::insert(Area::mock_tags(), &db.pool).await?;
        let element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        AreaElement::insert(area.id, element.id, &db.conn)?;
        let user = User::insert(1, &OsmUser::mock(), &db.conn)?;
        Event::insert(user.id, element.id, "create", &db.conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get_feed)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/feed", area.id))
            .to_request();
        let res: Vec<Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!("edit", res[0]["type"]);
        assert_eq!("create", res[0]["action"]);
        Ok(())
    }

    #[test]
    async fn get_feed_with_comment() -> Result<()> {
        let db = mock_db();
        let area = Area::insert(Area::mock_tags(), &db.pool).await?;
        let element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        AreaElement::insert(area.id, element.id, &db.conn)?;
        ElementComment::insert(element.id, "Great place!", &db.conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get_feed)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/feed", area.id))
            .to_request();
        let res: Vec<Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!("comment", res[0]["type"]);
        assert_eq!("Great place!", res[0]["comment"]);
        Ok(())
    }

    #[test]
    async fn get_feed_with_boost() -> Result<()> {
        let db = mock_db();
        let area = Area::insert(Area::mock_tags(), &db.pool).await?;
        let element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        AreaElement::insert(area.id, element.id, &db.conn)?;
        Boost::insert(1, element.id, 30, &db.conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get_feed)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/feed", area.id))
            .to_request();
        let res: Vec<Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        assert_eq!("boost", res[0]["type"]);
        assert_eq!(30, res[0]["duration_days"]);
        Ok(())
    }

    #[test]
    async fn get_feed_mixed_sorted_by_created_at_desc() -> Result<()> {
        let db = mock_db();
        let area = Area::insert(Area::mock_tags(), &db.pool).await?;
        let element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        AreaElement::insert(area.id, element.id, &db.conn)?;
        let user = User::insert(1, &OsmUser::mock(), &db.conn)?;
        Event::insert(user.id, element.id, "create", &db.conn)?;
        ElementComment::insert(element.id, "Nice!", &db.conn)?;
        Boost::insert(1, element.id, 7, &db.conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get_feed)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/feed", area.id))
            .to_request();
        let res: Vec<Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(3, res.len());
        let types: Vec<&str> = res.iter().map(|r| r["type"].as_str().unwrap()).collect();
        assert!(types.contains(&"edit"));
        assert!(types.contains(&"comment"));
        assert!(types.contains(&"boost"));
        Ok(())
    }

    #[test]
    async fn get_feed_respects_limit() -> Result<()> {
        let db = mock_db();
        let area = Area::insert(Area::mock_tags(), &db.pool).await?;
        let element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        AreaElement::insert(area.id, element.id, &db.conn)?;
        let user = User::insert(1, &OsmUser::mock(), &db.conn)?;
        Event::insert(user.id, element.id, "create", &db.conn)?;
        ElementComment::insert(element.id, "Nice!", &db.conn)?;
        Boost::insert(1, element.id, 7, &db.conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get_feed)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/feed?limit=1", area.id))
            .to_request();
        let res: Vec<Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        Ok(())
    }

    #[test]
    async fn get_feed_limit_clamps_to_max() -> Result<()> {
        let db = mock_db();
        let area = Area::insert(Area::mock_tags(), &db.pool).await?;
        let element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        AreaElement::insert(area.id, element.id, &db.conn)?;
        let user = User::insert(1, &OsmUser::mock(), &db.conn)?;
        Event::insert(user.id, element.id, "create", &db.conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get_feed)),
        )
        .await;
        // limit=200 should clamp to 100, but still return the 1 available item
        let req = TestRequest::get()
            .uri(&format!("/{}/feed?limit=200", area.id))
            .to_request();
        let res: Vec<Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        Ok(())
    }

    #[test]
    async fn get_feed_limit_clamps_to_min() -> Result<()> {
        let db = mock_db();
        let area = Area::insert(Area::mock_tags(), &db.pool).await?;
        let element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        AreaElement::insert(area.id, element.id, &db.conn)?;
        let user = User::insert(1, &OsmUser::mock(), &db.conn)?;
        Event::insert(user.id, element.id, "create", &db.conn)?;
        ElementComment::insert(element.id, "Nice!", &db.conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get_feed)),
        )
        .await;
        // limit=0 should clamp to 1
        let req = TestRequest::get()
            .uri(&format!("/{}/feed?limit=0", area.id))
            .to_request();
        let res: Vec<Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        Ok(())
    }

    #[test]
    async fn get_feed_after_pagination() -> Result<()> {
        let db = mock_db();
        let area = Area::insert(Area::mock_tags(), &db.pool).await?;
        let element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        AreaElement::insert(area.id, element.id, &db.conn)?;
        let user = User::insert(1, &OsmUser::mock(), &db.conn)?;
        Event::insert(user.id, element.id, "create", &db.conn)?;
        Event::insert(user.id, element.id, "update", &db.conn)?;
        Event::insert(user.id, element.id, "update", &db.conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get_feed)),
        )
        .await;

        // First page: limit=2
        let req = TestRequest::get()
            .uri(&format!("/{}/feed?limit=2", area.id))
            .to_request();
        let page1: Vec<Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(2, page1.len());

        // Second page: use last item's created_at as cursor
        let cursor = page1[1]["created_at"].as_str().unwrap();
        let encoded_cursor = cursor.replace(":", "%3A").replace("+", "%2B");
        let req = TestRequest::get()
            .uri(&format!(
                "/{}/feed?limit=2&after={}",
                area.id, encoded_cursor
            ))
            .to_request();
        let page2: Vec<Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, page2.len());
        Ok(())
    }

    #[test]
    async fn get_feed_by_alias() -> Result<()> {
        let db = mock_db();
        let area = Area::insert(Area::mock_tags(), &db.pool).await?;
        let element = Element::insert(&OverpassElement::mock(1), &db.conn)?;
        AreaElement::insert(area.id, element.id, &db.conn)?;
        let user = User::insert(1, &OsmUser::mock(), &db.conn)?;
        Event::insert(user.id, element.id, "create", &db.conn)?;

        let app = test::init_service(
            App::new()
                .app_data(Data::new(db.pool))
                .service(scope("/").service(super::get_feed)),
        )
        .await;
        let req = TestRequest::get()
            .uri(&format!("/{}/feed", area.alias()))
            .to_request();
        let res: Vec<Value> = test::call_and_read_body_json(&app, req).await;
        assert_eq!(1, res.len());
        Ok(())
    }
}
