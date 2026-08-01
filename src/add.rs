use anyhow::Result;
use crate::parser;
use crate::yaml;
use crate::file;
use crate::hash;

pub fn create_task(name: &String, depends_on: Vec<String>, run: Option<String>) -> Result<()> {
    let run_cmd = run.unwrap_or_else(|| format!("echo \"Running {}...\"", name));
    let deps = if depends_on.is_empty() { None } else { Some(depends_on) };

    let task_config = parser::Task {
        name: name.clone(),
        run: run_cmd,
        depends_on: deps,
    };

    yaml::writer::save_yaml(&format!("bee/tasks/{}.yml", name), &task_config)?;
    yaml::modify::append_to_yaml_list(&String::from("bee/system/config.yml"), &String::from("tasks"), name)?;

    let config_hash = hash::hash_string(&file::get_file_content(&String::from("bee/system/config.yml"))?);
    hash::write_hash_file("bee/system/hash/config", &config_hash)?;

    println!("Task '{}' created", name);
    Ok(())
}

pub fn create_pipeline(name: &String, tasks: Vec<String>) -> Result<()> {
    let pipeline_config = parser::Pipeline {
        name: name.clone(),
        tasks,
    };

    yaml::writer::save_yaml(&format!("bee/pipelines/{}.yml", name), &pipeline_config)?;
    yaml::modify::append_to_yaml_list(&String::from("bee/system/config.yml"), &String::from("pipelines"), name)?;

    let config_hash = hash::hash_string(&file::get_file_content(&String::from("bee/system/config.yml"))?);
    hash::write_hash_file("bee/system/hash/config", &config_hash)?;

    println!("Pipeline '{}' created", name);
    Ok(())
}

pub fn create_rule(name: &String, task: &String) -> Result<()> {
    let rule_config = parser::Rule {
        name: name.clone(),
        task: task.clone(),
        triggers: vec![parser::RuleTrigger::Manual],
    };

    yaml::writer::save_yaml(&format!("bee/rules/{}.yml", name), &rule_config)?;
    yaml::modify::append_to_yaml_list(&String::from("bee/system/config.yml"), &String::from("rules"), name)?;

    let config_hash = hash::hash_string(&file::get_file_content(&String::from("bee/system/config.yml"))?);
    hash::write_hash_file("bee/system/hash/config", &config_hash)?;

    println!("Rule '{}' created", name);
    Ok(())
}
