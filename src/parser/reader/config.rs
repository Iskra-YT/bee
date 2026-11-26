use crate::parser::{
    Dependency, DepsConfig, Pipeline, PipelineConfig, Rule, RuleConfig, Task, TaskConfig,
};
use crate::{file, yaml};
use anyhow::Result;

pub fn read_pipelines_config() -> Result<Vec<PipelineConfig>> {
    let yaml_content = file::get_file_content(&String::from("./bee/system/config.yml"))?;
    Ok(yaml::reader::parse_yaml_file(&yaml_content)?
        .get("pipelines")
        .and_then(|p| p.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| item.as_str())
                .map(|name| PipelineConfig {
                    name: name.to_string(),
                })
                .collect()
        })
        .unwrap_or_else(std::vec::Vec::new))
}

pub fn read_tasks_config() -> Result<Vec<TaskConfig>> {
    let yaml_content = file::get_file_content(&String::from("./bee/system/config.yml"))?;
    Ok(yaml::reader::parse_yaml_file(&yaml_content)?
        .get("tasks")
        .and_then(|p| p.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| item.as_str())
                .map(|name| TaskConfig {
                    name: name.to_string(),
                })
                .collect()
        })
        .unwrap_or_else(std::vec::Vec::new))
}

pub fn read_rules_config() -> Result<Vec<RuleConfig>> {
    let yaml_content = file::get_file_content(&String::from("./bee/system/config.yml"))?;
    Ok(yaml::reader::parse_yaml_file(&yaml_content)?
        .get("rules")
        .and_then(|p| p.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| item.as_str())
                .map(|name| RuleConfig {
                    name: name.to_string(),
                })
                .collect()
        })
        .unwrap_or_else(std::vec::Vec::new))
}

pub fn read_pipelines() -> Result<Vec<Pipeline>> {
    let pipelines = read_pipelines_config()?;
    let mut res: Vec<Pipeline> = vec![];

    for pipeline in pipelines {
        let pipeline_file =
            file::get_file_content(&format!("bee/pipelines/{}.yml", pipeline.name))?;
        let pipeline_yaml = yaml::reader::parse_yaml_file(&pipeline_file)?;

        if let Some(tasks) = pipeline_yaml.get("tasks").and_then(|t| t.as_sequence()) {
            let task_names = tasks
                .iter()
                .filter_map(|t| t.as_str())
                .map(|s| s.to_string())
                .collect();
            let subpipelines = pipeline_yaml
                .get("pipelines")
                .and_then(|p| p.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|p| p.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_else(std::vec::Vec::new);
            res.push(Pipeline {
                name: pipeline.name,
                tasks: task_names,
                pipelines: subpipelines,
            });
        }
    }

    Ok(res)
}

pub fn read_pipeline_from_string(content: &str) -> Result<Pipeline> {
    let pipeline_yaml = yaml::reader::parse_yaml_file(content)?;

    if let Some(name) = pipeline_yaml.get("name").and_then(|n| n.as_str()) {
        if let Some(tasks) = pipeline_yaml.get("tasks").and_then(|t| t.as_sequence()) {
            let task_names = tasks
                .iter()
                .filter_map(|t| t.as_str())
                .map(|s| s.to_string())
                .collect();
            let subpipelines = pipeline_yaml
                .get("pipelines")
                .and_then(|p| p.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|p| p.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_else(std::vec::Vec::new);
            return Ok(Pipeline {
                name: name.to_string(),
                tasks: task_names,
                pipelines: subpipelines,
            });
        }
    }

    Err(anyhow::anyhow!("Invalid pipeline YAML format"))
}

pub fn read_pipeline(name: &str) -> Result<Pipeline> {
    let pipeline_file = file::get_file_content(&format!("bee/pipelines/{}.yml", name))?;
    read_pipeline_from_string(&pipeline_file)
}
pub fn read_task_from_string(content: &str, name: &str) -> Result<Task> {
    let mut task: Task = serde_yaml::from_str(content)
        .map_err(|e| anyhow::anyhow!("Failed to parse task '{}': {}", name, e))?;
    if task.name.is_empty() {
        task.name = name.to_string();
    }
    task.validate()
        .map_err(|e| anyhow::anyhow!("Invalid task '{}': {}", name, e))?;
    Ok(task)
}

pub fn read_rules_from_string(content: &str) -> Result<Rule> {
    if let Ok(rule) = serde_yaml::from_str::<Rule>(content) {
        return Ok(rule);
    }

    Err(anyhow::anyhow!("Invalid rule YAML format"))
}

pub fn read_rules() -> Result<Vec<Rule>> {
    let rules_config = read_rules_config()?;
    let mut rules = Vec::new();

    for rule_config in rules_config {
        let rule_file = format!("bee/rules/{}.yml", rule_config.name);
        if let Ok(content) = file::get_file_content(&String::from(&rule_file)) {
            if let Ok(rule) = read_rules_from_string(&content) {
                rules.push(rule);
            }
        }
    }

    Ok(rules)
}

pub fn read_deps() -> Result<Vec<Dependency>> {
    let content = file::get_file_content(&String::from("bee/system/deps.yml"))?;
    let deps_config: DepsConfig = serde_yaml::from_str(&content)?;
    Ok(deps_config.dependencies)
}

pub fn read_deps_config() -> Result<Vec<Dependency>> {
    read_deps()
}
