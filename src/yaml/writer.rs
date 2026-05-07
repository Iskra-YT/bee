use anyhow::Result;
use crate::file;

pub fn save_yaml<T: serde::Serialize>(path: &str, data: &T) -> Result<()> {
    let yaml = serde_yaml::to_string(data)?;
    file::write_file_content(&String::from(path), &yaml)?;
    Ok(())
}