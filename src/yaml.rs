use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::file;

#[derive(Debug, Deserialize, Serialize)]
pub struct Task {
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub run: String
}

type TaskConfig = HashMap<String, Task>;

fn parse_yaml_file(value: &String) -> Result<TaskConfig> {
    let config: TaskConfig = serde_yaml::from_str(value)?;
    return Ok(config);
}

pub fn parse_yaml_files(path: &String) -> Result<Vec<TaskConfig>> {
    let dir = file::get_directory(path);
    let mut res = Vec::<TaskConfig>::new();

    for file in dir {
        let file = file?;
        let path = file.path();
        if !file.metadata()?.is_dir() && file.path().extension().and_then(|e| e.to_str()) != Some("yml") { continue };

        let content = file::get_file_content(&String::from(path.to_str().unwrap()));
        res.push(parse_yaml_file(&content).unwrap());
    }

    return Ok(res);
}