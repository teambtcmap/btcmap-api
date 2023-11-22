use crate::{area::Area, Error, Result};
use rusqlite::Connection;
use serde_json::{Map, Value};
use std::io::{stdin, stdout, Write};

pub async fn run(conn: &Connection) -> Result<()> {
    println!("Adding area");
    let mut tags = Map::new();

    print!("url_alias ");
    stdout().flush().unwrap();
    let mut url_alias = String::new();
    stdin().read_line(&mut url_alias).unwrap();
    let url_alias = url_alias.trim();
    tags.insert("url_alias".into(), Value::String(url_alias.into()));

    print!("name ");
    stdout().flush().unwrap();
    let mut name = String::new();
    stdin().read_line(&mut name).unwrap();
    let name = name.trim();
    tags.insert("name".into(), Value::String(name.into()));

    print!("geo_json ");
    stdout().flush().unwrap();
    let mut geo_json = String::new();
    stdin().read_line(&mut geo_json).unwrap();
    let geo_json: Map<String, Value> = serde_json::from_str(geo_json.trim())?;
    tags.insert("geo_json".into(), Value::Object(geo_json));

    print!("type ");
    stdout().flush().unwrap();
    let mut r#type = String::new();
    stdin().read_line(&mut r#type).unwrap();
    let r#type = r#type.trim();
    tags.insert("type".into(), Value::String(r#type.into()));

    print!("continent ");
    stdout().flush().unwrap();
    let mut continent = String::new();
    stdin().read_line(&mut continent).unwrap();
    let continent = continent.trim();
    tags.insert("continent".into(), Value::String(continent.into()));

    print!("contact:website ");
    stdout().flush().unwrap();
    let mut contact_website = String::new();
    stdin().read_line(&mut contact_website).unwrap();
    let contact_website = contact_website.trim();
    if contact_website.len() > 0 {
        tags.insert(
            "contact:website".into(),
            Value::String(contact_website.into()),
        );
    }

    print!("contact:telegram ");
    stdout().flush().unwrap();
    let mut contact_telegram = String::new();
    stdin().read_line(&mut contact_telegram).unwrap();
    let contact_telegram = contact_telegram.trim();
    if contact_telegram.len() > 0 {
        tags.insert(
            "contact:telegram".into(),
            Value::String(contact_telegram.into()),
        );
    }

    print!("contact:twitter ");
    stdout().flush().unwrap();
    let mut contact_twitter = String::new();
    stdin().read_line(&mut contact_twitter).unwrap();
    let contact_twitter = contact_twitter.trim();
    if contact_twitter.len() > 0 {
        tags.insert(
            "contact:twitter".into(),
            Value::String(contact_twitter.into()),
        );
    }

    print!("Enter icon file format ");
    stdout().flush().unwrap();
    let mut icon_square_ext = String::new();
    stdin().read_line(&mut icon_square_ext).unwrap();
    let icon_square_ext = icon_square_ext.trim();
    tags.insert(
        "icon:square".into(),
        Value::String(format!(
            "https://static.btcmap.org/images/communities/{}.{}",
            url_alias, icon_square_ext,
        )),
    );

    match Area::select_by_url_alias(url_alias, conn)? {
        Some(_) => Err(Error::HttpConflict(
            "Area with this url_alias already exists".into(),
        ))?,
        None => Area::insert(&tags, conn)?,
    };

    Ok(())
}
