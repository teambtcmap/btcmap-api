use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use rusqlite::Connection;
use serde::Deserialize;
use serde_json::Value;
use tracing::info;

use crate::{model::Area, Error, Result};

#[derive(Deserialize)]
struct CountryJson {
    id: String,
    tags: HashMap<String, Value>,
}

pub fn run(path: &str, conn: &mut Connection) -> Result<()> {
    let path = Path::new(path);

    if !path.try_exists().is_ok_and(|it| it == true) {
        return Err(Error::Other(format!("Path doesnt exist: {path:?}")))?;
    }

    info!(path = path.to_str(), "Given path is correct");

    if !path.is_dir() {
        return Err(Error::Other(format!("Path is not a directory: {path:?}")))?;
    }

    let tx = conn.transaction()?;

    for dir_entry in path.read_dir().expect("Failed to read files") {
        if let Ok(dir_entry) = dir_entry {
            if !dir_entry.path().is_file() {
                return Err(Error::Other(format!("Not a file: {:?}", dir_entry.path())))?;
            }

            if dir_entry.file_name().len() != 7 {
                Err(Error::Other(format!(
                    "Invalid file name: {:?}",
                    dir_entry.file_name()
                )))?;
            }

            let file = File::open(dir_entry.path())?;
            let reader = BufReader::new(file);
            let json: CountryJson = serde_json::from_reader(reader)?;

            match Area::select_by_url_alias(&json.id, &tx)? {
                Some(area) => {
                    area.patch_tags(&json.tags, &tx)?;
                    info!(json.id, "Patched tags for an existing area");
                }
                None => {
                    Area::insert(&json.tags, &tx)?;
                    info!(json.id, "Inserted area");
                }
            }
        }
    }

    tx.commit()?;

    Ok(())
}
