use crate::Result;
use std::{fs::create_dir_all, path::PathBuf};

pub fn data_dir_file_path(file_name: &str) -> Result<PathBuf> {
    #[allow(deprecated)]
    let data_dir = std::env::home_dir()
        .ok_or("Home directory does not exist")?
        .join(".local/share/btcmap");
    if !data_dir.exists() {
        create_dir_all(&data_dir)?;
    }
    Ok(data_dir.join(file_name))
}
