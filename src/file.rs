use std::fs::{self, ReadDir};
use anyhow::Result;

pub fn get_file_content(path: &String) -> Result<String> {
    return Ok(fs::read_to_string(path)?);
}

pub fn get_directory(path: &String) -> Result<ReadDir> {
    return Ok(fs::read_dir(path)?);
}

pub fn write_file_content(path: &String, content: &String) -> Result<()> {
    fs::write(path, content)?;
    Ok(())
}