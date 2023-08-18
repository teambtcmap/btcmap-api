use std::io::{stdin, stdout, Write};

use crate::Result;
use rusqlite::Connection;

pub async fn run(db: &mut Connection) -> Result<()> {
    println!("Adding area");

    print!("Enter area ID: ");
    stdout().flush().unwrap();
    let mut area_id = String::new();
    stdin().read_line(&mut area_id).unwrap();
    let area_id = area_id.trim();

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

    let tx = db.transaction().unwrap();

    tx.execute("INSERT INTO area (id) VALUES (?)", [&area_id])
        .unwrap();

    tx.execute(
        "UPDATE AREA SET tags = json_set(tags, '$.name', ?) WHERE id = ?",
        [&area_name, &area_id],
    )
    .unwrap();

    tx.execute(
        "UPDATE AREA SET tags = json_set(tags, '$.geo_json', json(?)) WHERE id = ?",
        [&geo_json, &area_id],
    )
    .unwrap();

    tx.execute(
        "UPDATE AREA SET tags = json_set(tags, '$.icon:square', ?) WHERE id = ?",
        [
            format!(
                "https://static.btcmap.org/images/communities/{}.{}",
                area_id, icon_square_ext,
            ),
            area_id.into(),
        ],
    )
    .unwrap();

    tx.execute(
        "UPDATE AREA SET tags = json_set(tags, '$.type', ?) WHERE id = ?",
        [&area_type, &area_id],
    )
    .unwrap();

    tx.execute(
        "UPDATE AREA SET tags = json_set(tags, '$.continent', ?) WHERE id = ?",
        [&area_continent, &area_id],
    )
    .unwrap();

    if contact_website.len() > 0 {
        tx.execute(
            "UPDATE AREA SET tags = json_set(tags, '$.contact:website', ?) WHERE id = ?",
            [&contact_website, &area_id],
        )
        .unwrap();
    }

    if contact_telegram.len() > 0 {
        tx.execute(
            "UPDATE AREA SET tags = json_set(tags, '$.contact:telegram', ?) WHERE id = ?",
            [&contact_telegram, &area_id],
        )
        .unwrap();
    }

    if contact_twitter.len() > 0 {
        tx.execute(
            "UPDATE AREA SET tags = json_set(tags, '$.contact:twitter', ?) WHERE id = ?",
            [&contact_twitter, &area_id],
        )
        .unwrap();
    }

    tx.commit().unwrap();

    Ok(())
}
