use crate::area::Area;
use crate::area_element::model::AreaElement;
use crate::element_comment::ElementComment;
use crate::Result;
use crate::{element::Element, event::Event};
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse, Responder,
};
use deadpool_sqlite::Pool;
use std::collections::HashSet;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

#[get("/new-places")]
async fn new_places(pool: Data<Pool>) -> Result<impl Responder> {
    let events: Vec<(Event, Element)> = pool
        .get()
        .await?
        .interact(move |conn| {
            Event::select_by_type("create", Some("DESC".into()), Some(100), conn)
                .unwrap()
                .into_iter()
                .map(|it| {
                    let cloned_element_id = it.element_id;
                    (
                        it,
                        Element::select_by_id(cloned_element_id, conn)
                            .unwrap()
                            .unwrap(),
                    )
                })
                .collect()
        })
        .await?;
    Ok(HttpResponse::Ok()
        .insert_header(("content-type", "application/atom+xml; charset=utf-8"))
        .body(events_to_atom_feed(
            "https://api.btcmap.org/feeds/new-places",
            "BTC Map - New Places",
            events,
        )))
}

#[get("/new-places/{area}")]
async fn new_places_for_area(area: Path<String>, pool: Data<Pool>) -> Result<impl Responder> {
    let area = pool
        .get()
        .await?
        .interact(move |conn| Area::select_by_id_or_alias(&area, conn))
        .await??
        .unwrap();
    let area_elements = pool
        .get()
        .await?
        .interact(move |conn| AreaElement::select_by_area_id(area.id, conn))
        .await??;
    let area_element_ids: HashSet<i64> =
        area_elements.into_iter().map(|it| it.element_id).collect();
    let mut events: Vec<(Event, Element)> = pool
        .get()
        .await?
        .interact(move |conn| {
            Event::select_updated_since(
                &OffsetDateTime::now_utc()
                    .checked_sub(Duration::days(180))
                    .unwrap(),
                None,
                conn,
            )
            .unwrap()
            .into_iter()
            .filter(|it| it.r#type == "create" && area_element_ids.contains(&it.element_id))
            .map(|it| {
                let cloned_element_id = it.element_id;
                (
                    it,
                    Element::select_by_id(cloned_element_id, conn)
                        .unwrap()
                        .unwrap(),
                )
            })
            .collect()
        })
        .await?;
    events.sort_by(|a, b| b.0.updated_at.cmp(&a.0.updated_at));
    Ok(HttpResponse::Ok()
        .insert_header(("content-type", "application/atom+xml; charset=utf-8"))
        .body(events_to_atom_feed(
            &format!("https://api.btcmap.org/feeds/new-places?area={}", area.id),
            &format!("BTC Map - New Places in {}", area.name()),
            events,
        )))
}

fn events_to_atom_feed(feed_id: &str, feed_title: &str, events: Vec<(Event, Element)>) -> String {
    let feed_title = xml_escape(feed_title.into());
    let mut res = String::new();
    res.push_str(r#"<?xml version="1.0" encoding="utf-8"?>"#);
    res.push_str(r#"<feed xmlns="http://www.w3.org/2005/Atom">"#);
    res.push_str(&format!(r#"<id>{feed_id}</id>"#));
    res.push_str(&format!(r#"<title type="text">{feed_title}</title>"#));
    res.push_str(r#"<link rel="alternate" type="text/html" href="https://btcmap.org"/>"#);
    res.push_str(&format!(
        r#"<link rel="self" type="application/atom+xml" href="{feed_id}"/>"#
    ));
    res.push_str(&format!(
        r#"<updated>{}</updated>"#,
        OffsetDateTime::now_utc().format(&Rfc3339).unwrap()
    ));
    for event in events {
        res.push_str(&event_to_atom_entry(event));
    }
    res.push_str(r#"</feed>"#);
    res
}

fn event_to_atom_entry(event: (Event, Element)) -> String {
    let event_id = event.0.id;
    let event_created_at = event.0.created_at.format(&Rfc3339).unwrap();
    let element_id = event.1.overpass_data.btcmap_id();
    let title = format!("{}", xml_escape(event.1.name()));
    let summary = format!("Check BTC Map for more details");
    format!(
        r#"
            <entry>
                <id>https://btcmap.org/event/{event_id}</id>
                <title>{title}</title>
                <author><name>BTC Map</name></author>
                <updated>{event_created_at}</updated>
                <summary type="text">{summary}</summary>
                <link rel="alternate" type="text/html" href="https://btcmap.org/merchant/{element_id}"/>
            </entry>
        "#
    )
    .into()
}

#[get("/new-comments")]
async fn new_comments(pool: Data<Pool>) -> Result<impl Responder> {
    let comments: Vec<(ElementComment, Element)> = pool
        .get()
        .await?
        .interact(move |conn| {
            ElementComment::select_latest(100, conn)
                .unwrap()
                .into_iter()
                .map(|it| {
                    let cloned_element_id = it.element_id;
                    (
                        it,
                        Element::select_by_id(cloned_element_id, conn)
                            .unwrap()
                            .unwrap(),
                    )
                })
                .collect()
        })
        .await?;
    Ok(HttpResponse::Ok()
        .insert_header(("content-type", "application/atom+xml; charset=utf-8"))
        .body(comments_to_atom_feed(
            "https://api.btcmap.org/feeds/new-comments",
            "BTC Map - New Comments",
            comments,
        )))
}

#[get("/new-comments/{area}")]
async fn new_comments_for_area(area: Path<String>, pool: Data<Pool>) -> Result<impl Responder> {
    let area = pool
        .get()
        .await?
        .interact(move |conn| Area::select_by_id_or_alias(&area, conn))
        .await??
        .unwrap();
    let area_id = area.id;
    let area_name = area.name();
    let comments: Vec<(ElementComment, Element)> = pool
        .get()
        .await?
        .interact(move |conn| {
            crate::area::service::get_comments(&area, conn)
                .unwrap()
                .into_iter()
                .map(|it| {
                    let cloned_element_id = it.element_id;
                    (
                        it,
                        Element::select_by_id(cloned_element_id, conn)
                            .unwrap()
                            .unwrap(),
                    )
                })
                .collect()
        })
        .await?;
    Ok(HttpResponse::Ok()
        .insert_header(("content-type", "application/atom+xml; charset=utf-8"))
        .body(comments_to_atom_feed(
            &format!("https://api.btcmap.org/feeds/new-comments?area={}", area_id),
            &format!("BTC Map - New Comments in {}", area_name),
            comments,
        )))
}

fn comments_to_atom_feed(
    feed_id: &str,
    feed_title: &str,
    comments: Vec<(ElementComment, Element)>,
) -> String {
    let feed_title = xml_escape(feed_title.into());
    let mut res = String::new();
    res.push_str(r#"<?xml version="1.0" encoding="utf-8"?>"#);
    res.push_str(r#"<feed xmlns="http://www.w3.org/2005/Atom">"#);
    res.push_str(&format!(r#"<id>{feed_id}</id>"#));
    res.push_str(&format!(r#"<title type="text">{feed_title}</title>"#));
    res.push_str(r#"<link rel="alternate" type="text/html" href="https://btcmap.org"/>"#);
    res.push_str(&format!(
        r#"<link rel="self" type="application/atom+xml" href="{feed_id}"/>"#
    ));
    res.push_str(&format!(
        r#"<updated>{}</updated>"#,
        OffsetDateTime::now_utc().format(&Rfc3339).unwrap()
    ));
    for comment in comments {
        res.push_str(&comment_to_atom_entry(comment));
    }
    res.push_str(r#"</feed>"#);
    res
}

fn comment_to_atom_entry(comment: (ElementComment, Element)) -> String {
    let comment_id = comment.0.id;
    let comment_created_at = comment.0.created_at.format(&Rfc3339).unwrap();
    let element_id = comment.1.overpass_data.btcmap_id();
    let title = format!("{}", xml_escape(comment.0.comment.clone()));
    let summary = xml_escape(comment.0.comment);
    format!(
        r#"
            <entry>
                <id>https://btcmap.org/comment/{comment_id}</id>
                <title>{title}</title>
                <author><name>BTC Map</name></author>
                <updated>{comment_created_at}</updated>
                <summary type="text">{summary}</summary>
                <link rel="alternate" type="text/html" href="https://btcmap.org/merchant/{element_id}"/>
            </entry>
        "#
    )
    .into()
}

fn xml_escape(str: String) -> String {
    str.replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace(r#"""#, "&quot;")
        .replace(r#"\"#, "&apos;")
        .replace("&", "&amp;")
}
