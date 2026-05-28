use anyhow::Result;
use crate::{file, hash};
use crate::parser::reader::config as parser;

fn backup_names() -> Result<()> {
    let mut json = json::object! {};

    let pipelines = parser::read_pipelines()?;
    for pipe in pipelines {
        json[hash::hash_string_with_salt(&pipe.name, "pipeline").chars().take(8).collect::<String>()] = json::JsonValue::String(pipe.name.clone());
    }

    let tasks = parser::read_tasks_config()?;
    for task in tasks {
        json[hash::hash_string_with_salt(&task.name, "task").chars().take(8).collect::<String>()] = json::JsonValue::String(task.name.clone());
    }

    let rules = parser::read_rules_config()?;
    for rule in rules {
        json[hash::hash_string_with_salt(&rule.name, "rule").chars().take(8).collect::<String>()] = json::JsonValue::String(rule.name.clone());
    }

    file::write_file_content(&String::from("bee/system/state/names.json"), &json::stringify(json))?;

    let names_content = file::get_file_content(&String::from("bee/system/state/names.json"))?;
    file::write_file_content(&String::from("bee/system/hash/state/names"), &hash::hash_string(&names_content))?;
    Ok(())
}

pub fn make_backup() -> Result<()> {
    backup_names()?;
    Ok(())
}
