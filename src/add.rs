use crate::file;
use crate::hash;
use crate::parser;
use crate::yaml;
use anyhow::Result;

#[allow(clippy::too_many_arguments)]
pub fn create_task(
    name: &String,
    depends_on: Vec<String>,
    run: Option<String>,
    executor: Option<String>,
    program: Option<String>,
    args: Vec<String>,
    working_dir: Option<String>,
    shell: Option<String>,
    interpreter: Option<String>,
    script: Option<String>,
) -> Result<()> {
    let deps = if depends_on.is_empty() {
        None
    } else {
        Some(depends_on)
    };
    let has_explicit = executor.is_some()
        || program.is_some()
        || !args.is_empty()
        || working_dir.is_some()
        || shell.is_some()
        || interpreter.is_some()
        || script.is_some();
    let run_cmd = if run.is_some() || has_explicit {
        run
    } else {
        Some(format!("echo \"Running {}...\"", name))
    };

    let task_config = parser::Task {
        name: name.clone(),
        run: run_cmd,
        depends_on: deps,
        executor,
        program,
        args: if args.is_empty() { None } else { Some(args) },
        shell,
        interpreter,
        interpreter_candidates: None,
        script,
        working_dir,
        env: None,
        platforms: None,
        legacy: false,
    };

    task_config
        .validate()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    yaml::writer::save_yaml(&format!("bee/tasks/{}.yml", name), &task_config)?;
    yaml::modify::append_to_yaml_list(
        &String::from("bee/system/config.yml"),
        &String::from("tasks"),
        name,
    )?;

    let config_hash = hash::hash_string(&file::get_file_content(&String::from(
        "bee/system/config.yml",
    ))?);
    hash::write_hash_file("bee/system/hash/config", &config_hash)?;

    println!("Task '{}' created", name);
    Ok(())
}

pub fn create_pipeline(name: &String, tasks: Vec<String>, pipelines: Vec<String>) -> Result<()> {
    let pipeline_config = parser::Pipeline {
        name: name.clone(),
        tasks,
        pipelines,
    };

    yaml::writer::save_yaml(&format!("bee/pipelines/{}.yml", name), &pipeline_config)?;
    yaml::modify::append_to_yaml_list(
        &String::from("bee/system/config.yml"),
        &String::from("pipelines"),
        name,
    )?;

    let config_hash = hash::hash_string(&file::get_file_content(&String::from(
        "bee/system/config.yml",
    ))?);
    hash::write_hash_file("bee/system/hash/config", &config_hash)?;

    println!("Pipeline '{}' created", name);
    Ok(())
}

pub fn create_rule(name: &String, task: &str) -> Result<()> {
    let rule_config = parser::Rule {
        name: name.clone(),
        task: task.to_owned(),
        triggers: vec![parser::RuleTrigger::Manual],
    };

    yaml::writer::save_yaml(&format!("bee/rules/{}.yml", name), &rule_config)?;
    yaml::modify::append_to_yaml_list(
        &String::from("bee/system/config.yml"),
        &String::from("rules"),
        name,
    )?;

    let config_hash = hash::hash_string(&file::get_file_content(&String::from(
        "bee/system/config.yml",
    ))?);
    hash::write_hash_file("bee/system/hash/config", &config_hash)?;

    println!("Rule '{}' created", name);
    Ok(())
}
