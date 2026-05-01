use anyhow::Result;
use std::fs;
use std::collections::HashMap;
use crate::file;

pub fn parse_yaml_file(value: &String) -> Result<serde_yaml::Value> {
    Ok(serde_yaml::from_str(value)?)
}

pub fn parse_yaml_files(path: &String) -> Result<HashMap<String, serde_yaml::Value>> {
    let folder = file::get_directory(path)?;
    let mut res: HashMap<String, serde_yaml::Value> = HashMap::new();

    for file in folder {
        let file = file?;
        let path = file.path();

        if file.metadata()?.is_dir() {
            continue;
        }

        if path.extension().and_then(|e| e.to_str()) != Some("yml") {
            continue;
        }

        let content = fs::read_to_string(&path)?;
        let yaml = parse_yaml_file(&content)?;

        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            res.insert(stem.to_string(), yaml);
        }
    }

    Ok(res)
}