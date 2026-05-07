use crate::hash;
use anyhow::Result;
use std::fs;

pub fn get_file_content(path: &String) -> Result<String> {
    return Ok(fs::read_to_string(path)?);
}

pub fn write_file_content(path: &String, content: &String) -> Result<()> {
    fs::write(path, content)?;
    Ok(())
}

pub fn check_bee_directory() -> bool {
    let init_content = get_file_content(&String::from("bee/system/init"));
    let init_hash = get_file_content(&String::from("bee/system/hash/init"));
    fs::metadata("bee/system/init").is_ok()
        && hash::hash_string(init_content.unwrap_or_default().as_str())
            == init_hash.unwrap_or_default()
}
