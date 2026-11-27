use crate::file;
use crate::hash;
use crate::parser;
use crate::safety;
use crate::yaml;
use anyhow::{anyhow, Result};

fn registered_names(key: &str) -> Result<Vec<String>> {
    let config = file::get_file_content("bee/system/config.yml")?;
    let root = yaml::reader::parse_yaml_file(&config)?;
    root.get(key)
        .and_then(|value| value.as_sequence())
        .ok_or_else(|| anyhow!("bee/system/config.yml is missing the '{}' list", key))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| anyhow!("invalid entry in '{}' list: expected a name", key))
        })
        .collect()
}

fn ensure_new(kind: &str, name: &str, registry_key: &str) -> Result<()> {
    safety::validate_name(kind, name)?;
    if registered_names(registry_key)?.iter().any(|n| n == name) {
        return Err(anyhow!("{} '{}' already exists", kind, name));
    }
    let path = safety::safe_path(
        format!("bee/{}s", registry_key.trim_end_matches('s')),
        kind,
        name,
    )?
    .with_extension("yml");
    if path.exists() {
        return Err(anyhow!(
            "{} '{}' already has a definition file ('{}') but is not registered; add it manually or remove the file",
            kind,
            name,
            path.display()
        ));
    }
    Ok(())
}

fn ensure_references(kind: &str, references: &[String], registry_key: &str) -> Result<()> {
    let known = registered_names(registry_key)?;
    for reference in references {
        safety::validate_name(kind, reference)?;
        if !known.iter().any(|n| n == reference) {
            return Err(anyhow!(
                "{} '{}' does not exist; create it first with 'bee {} create {}'",
                kind,
                reference,
                kind,
                reference
            ));
        }
    }
    Ok(())
}

fn update_registry_hash() -> Result<()> {
    let config_hash = hash::hash_string(&file::get_file_content("bee/system/config.yml")?);
    hash::write_hash_file("bee/system/hash/config", &config_hash)?;
    Ok(())
}

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
    ensure_new("task", name, "tasks")?;
    ensure_references("task", &depends_on, "tasks")?;

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
        timeout: None,
        inherit_env: None,
        legacy: false,
    };

    task_config.validate()?;

    let path = safety::safe_path("bee/tasks", "task", name)?.with_extension("yml");
    yaml::writer::save_yaml(&path.to_string_lossy(), &task_config)?;
    yaml::modify::append_to_yaml_list("bee/system/config.yml", "tasks", name)?;
    update_registry_hash()?;

    println!("Task '{}' created", name);
    Ok(())
}

pub fn create_pipeline(name: &String, tasks: Vec<String>, pipelines: Vec<String>) -> Result<()> {
    ensure_new("pipeline", name, "pipelines")?;
    ensure_references("task", &tasks, "tasks")?;
    ensure_references("pipeline", &pipelines, "pipelines")?;

    let pipeline_config = parser::Pipeline {
        name: name.clone(),
        tasks,
        pipelines,
    };
    crate::parser::reader::config::read_pipeline_from_string(&serde_yaml::to_string(
        &pipeline_config,
    )?)?;

    let path = safety::safe_path("bee/pipelines", "pipeline", name)?.with_extension("yml");
    yaml::writer::save_yaml(&path.to_string_lossy(), &pipeline_config)?;
    yaml::modify::append_to_yaml_list("bee/system/config.yml", "pipelines", name)?;
    update_registry_hash()?;

    println!("Pipeline '{}' created", name);
    Ok(())
}

pub fn create_rule(name: &String, task: &str) -> Result<()> {
    ensure_new("rule", name, "rules")?;
    let task_refs = [task.to_string()];
    ensure_references("task", &task_refs, "tasks")?;

    let rule_config = parser::Rule {
        name: name.clone(),
        task: task.to_owned(),
        triggers: vec![parser::RuleTrigger::Manual],
    };
    crate::parser::reader::config::read_rules_from_string(&serde_yaml::to_string(
        &rule_config,
    )?)?;

    let path = safety::safe_path("bee/rules", "rule", name)?.with_extension("yml");
    yaml::writer::save_yaml(&path.to_string_lossy(), &rule_config)?;
    yaml::modify::append_to_yaml_list("bee/system/config.yml", "rules", name)?;
    update_registry_hash()?;

    println!("Rule '{}' created", name);
    Ok(())
}
