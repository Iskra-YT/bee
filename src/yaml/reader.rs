use anyhow::Result;
use std::fs;
use crate::file;

pub fn parse_yaml_file(value: &String) -> Result<serde_yaml::Value> {
    Ok(serde_yaml::from_str(value)?)
}

pub fn parse_yaml_files(path: &String) -> Result<Vec<serde_yaml::Value>> {
    let folder = file::get_directory(path)?;
    let mut res: Vec<serde_yaml::Value> = vec![];

    for file in folder {
        let file = file?;
        let path = file.path();
        if !file.metadata()?.is_dir() && file.path().extension().and_then(|e| e.to_str()) != Some("yml") { continue };
        let content = fs::read_to_string(path)?;
        let yaml = parse_yaml_file(&content)?;
        res.push(yaml);
    }

    Ok(res)
}