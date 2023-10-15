use std::{
    collections::HashMap,
    io::{stdin, stdout, Write},
};

use crate::{model::Area, Result};
use rusqlite::Connection;
use serde_json::Value;

pub async fn run(conn: &mut Connection) -> Result<()> {
    println!("Adding area");

    print!("Enter area url_alias: ");
    stdout().flush().unwrap();
    let mut url_alias = String::new();
    stdin().read_line(&mut url_alias).unwrap();
    let url_alias = url_alias.trim();

    print!("Enter area name: ");
    stdout().flush().unwrap();
    let mut area_name = String::new();
    stdin().read_line(&mut area_name).unwrap();
    let area_name = area_name.trim();

    print!("Enter GeoJSON: ");
    stdout().flush().unwrap();
    let mut geo_json = String::new();
    stdin().read_line(&mut geo_json).unwrap();
    let geo_json = geo_json.trim();

    print!("Enter icon URL extension: ");
    stdout().flush().unwrap();
    let mut icon_square_ext = String::new();
    stdin().read_line(&mut icon_square_ext).unwrap();
    let icon_square_ext = icon_square_ext.trim();

    print!("Enter area type: ");
    stdout().flush().unwrap();
    let mut area_type = String::new();
    stdin().read_line(&mut area_type).unwrap();
    let area_type = area_type.trim();

    print!("Enter area continent: ");
    stdout().flush().unwrap();
    let mut area_continent = String::new();
    stdin().read_line(&mut area_continent).unwrap();
    let area_continent = area_continent.trim();

    print!("Enter contact website URL: ");
    stdout().flush().unwrap();
    let mut contact_website = String::new();
    stdin().read_line(&mut contact_website).unwrap();
    let contact_website = contact_website.trim();

    print!("Enter contact Telegram URL: ");
    stdout().flush().unwrap();
    let mut contact_telegram = String::new();
    stdin().read_line(&mut contact_telegram).unwrap();
    let contact_telegram = contact_telegram.trim();

    print!("Enter contact Twitter URL: ");
    stdout().flush().unwrap();
    let mut contact_twitter = String::new();
    stdin().read_line(&mut contact_twitter).unwrap();
    let contact_twitter = contact_twitter.trim();

    let mut tags: HashMap<String, Value> = HashMap::new();
    tags.insert("name".into(), Value::String(area_name.into()));
    tags.insert("geo_json".into(), Value::String(geo_json.into()));
    tags.insert(
        "icon:square".into(),
        Value::String(format!(
            "https://static.btcmap.org/images/communities/{}.{}",
            url_alias, icon_square_ext,
        )),
    );
    tags.insert("type".into(), Value::String(area_type.into()));
    tags.insert("continent".into(), Value::String(area_continent.into()));

    if contact_website.len() > 0 {
        tags.insert(
            "contact:website".into(),
            Value::String(contact_website.into()),
        );
    }

    if contact_telegram.len() > 0 {
        tags.insert(
            "contact:telegram".into(),
            Value::String(contact_telegram.into()),
        );
    }

    if contact_twitter.len() > 0 {
        tags.insert(
            "contact:twitter".into(),
            Value::String(contact_twitter.into()),
        );
    }

    Area::insert_or_replace(url_alias, Some(&tags), &conn)?;

    Ok(())
}
