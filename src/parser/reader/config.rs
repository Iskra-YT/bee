use crate::parser::{Pipeline, PipelineConfig, Rule, RuleConfig, Task, TaskConfig};
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
                .map(|name| PipelineConfig { name: name.to_string() })
                .collect()
        })
        .unwrap_or_else(|| vec![]))
}

pub fn read_tasks_config() -> Result<Vec<TaskConfig>> {
    let yaml_content = file::get_file_content(&String::from("./bee/system/config.yml"))?;
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
    let yaml_content = file::get_file_content(&String::from("./bee/system/config.yml"))?;
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

pub fn read_pipelines() -> Result<Vec<Pipeline>> {
    let pipelines = read_pipelines_config()?;
    let mut res: Vec<Pipeline> = vec![];

    for pipeline in pipelines {
        let pipeline_file = file::get_file_content(&format!("bee/pipelines/{}.yml", pipeline.name))?;
        let pipeline_yaml = yaml::reader::parse_yaml_file(&pipeline_file)?;

        if let Some(tasks) = pipeline_yaml.get("tasks").and_then(|t| t.as_sequence()) {
            let task_names = tasks.iter().filter_map(|t| t.as_str()).map(|s| s.to_string()).collect();
            res.push(Pipeline { name: pipeline.name, tasks: task_names });
        }
    }

    Ok(res)
}

pub fn read_pipeline_from_string(content: &String) -> Result<Pipeline> {
    let pipeline_yaml = yaml::reader::parse_yaml_file(content)?;

    if let Some(name) = pipeline_yaml.get("name").and_then(|n| n.as_str()) {
        if let Some(tasks) = pipeline_yaml.get("tasks").and_then(|t| t.as_sequence()) {
            let task_names = tasks.iter().filter_map(|t| t.as_str()).map(|s| s.to_string()).collect();
            return Ok(Pipeline { name: name.to_string(), tasks: task_names });
        }
    }

    Err(anyhow::anyhow!("Invalid pipeline YAML format"))
}
pub fn read_task_from_string(content: &String, name: &String) -> Result<Task> {
    let task_yaml = yaml::reader::parse_yaml_file(content)?;

    if let Some(run) = task_yaml.get("run").and_then(|r| r.as_str()) {
        let depends_on = task_yaml.get("depends_on")
            .and_then(|d| d.as_sequence())
            .map(|seq| seq.iter().filter_map(|t| t.as_str()).map(|s| s.to_string()).collect())
            .unwrap_or_else(|| vec![]);

        return Ok(Task { 
            name: name.clone(), 
            run: run.to_string(), 
            depends_on: Some(depends_on)
        });
    }

    Err(anyhow::anyhow!("Invalid task YAML format"))
}

pub fn read_rules_from_string(content: &String) -> Result<Rule> {
    if let Ok(rule) = serde_yaml::from_str::<Rule>(content) {
        return Ok(rule);
    }

    Err(anyhow::anyhow!("Invalid rule YAML format"))
}