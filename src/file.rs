use std::fs;
use anyhow::Result;

pub fn get_file_content(path: &String) -> Result<String> {
    return Ok(fs::read_to_string(path)?);
}

pub fn write_file_content(path: &String, content: &String) -> Result<()> {
    fs::write(path, content)?;
    Ok(())
}

pub fn check_bee_directory() -> bool {
    fs::metadata("bee/cache/init").is_ok()
}