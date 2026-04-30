use anyhow::Result;
use std::fs;

pub fn parse_yaml_file(value: &String) -> Result<serde_yaml::Value> {
    return Ok(serde_yaml::from_str(value)?);
}

pub fn save_yaml_file(path: &str, value: &serde_yaml::Value) -> Result<()> {
    let yaml_string = serde_yaml::to_string(value)?;
    fs::write(path, yaml_string)?;
    Ok(())
}