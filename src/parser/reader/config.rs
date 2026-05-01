use crate::parser::{PipelineConfig, RuleConfig, TaskConfig};
use crate::{file, yaml};
use anyhow::Result;

pub fn read_pipelines_config() -> Result<Vec<PipelineConfig>> {
    let yaml_content = file::get_file_content(&String::from("./bee/config.yml"))?;
    Ok(yaml::reader::parse_yaml_file(&yaml_content)?
        .get("pipelines")
        .and_then(|p| p.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| item.as_str())
                .map(|name| PipelineConfig { name: name.to_string() })
                .collect()
        })
        .unwrap_or_else(|| vec![]))
}

pub fn read_tasks_config() -> Result<Vec<TaskConfig>> {
    let yaml_content = file::get_file_content(&String::from("./bee/config.yml"))?;
    Ok(yaml::reader::parse_yaml_file(&yaml_content)?
        .get("tasks")
        .and_then(|p| p.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| item.as_str())
                .map(|name| TaskConfig { name: name.to_string() })
                .collect()
        })
        .unwrap_or_else(|| vec![]))
}

pub fn read_rules_config() -> Result<Vec<RuleConfig>> {
    let yaml_content = file::get_file_content(&String::from("./bee/config.yml"))?;
    Ok(yaml::reader::parse_yaml_file(&yaml_content)?
        .get("rules")
        .and_then(|p| p.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| item.as_str())
                .map(|name| RuleConfig { name: name.to_string() })
                .collect()
        })
        .unwrap_or_else(|| vec![]))
}