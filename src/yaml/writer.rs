use anyhow::Result;
use std::fs;

pub fn save_yaml<T: serde::Serialize>(path: &str, data: &T) -> Result<()> {
    let yaml = serde_yaml::to_string(data)?;
    fs::write(path, yaml)?;
    Ok(())
}