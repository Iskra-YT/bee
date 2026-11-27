use crate::{file, hash, yaml};
use anyhow::Result;
use serde_yaml::Value;

#[allow(clippy::too_many_arguments)]
pub fn modify_task(
    name: &str,
    add_deps: &[String],
    rm_deps: &[String],
    run: Option<&str>,
    executor: Option<&str>,
    program: Option<&str>,
    args: &[String],
    working_dir: Option<&str>,
    shell: Option<&str>,
    interpreter: Option<&str>,
    script: Option<&str>,
) -> Result<()> {
    let path = format!("bee/tasks/{}.yml", name);
    let content = file::get_file_content(&String::from(&path))?;
    let mut doc: Value = serde_yaml::from_str(&content)?;
    let mapping = doc
        .as_mapping_mut()
        .ok_or_else(|| anyhow::anyhow!("Invalid task YAML"))?;

    if let Some(r) = run {
        mapping.insert(Value::String("run".into()), Value::String(r.into()));
    }
    if let Some(v) = executor {
        mapping.insert(Value::String("executor".into()), Value::String(v.into()));
    }
    if let Some(v) = program {
        mapping.insert(Value::String("program".into()), Value::String(v.into()));
    }
    if !args.is_empty() {
        let seq: Vec<Value> = args.iter().map(|a| Value::String(a.clone())).collect();
        mapping.insert(Value::String("args".into()), Value::Sequence(seq));
    }
    if let Some(v) = working_dir {
        mapping.insert(Value::String("working_dir".into()), Value::String(v.into()));
    }
    if let Some(v) = shell {
        mapping.insert(Value::String("shell".into()), Value::String(v.into()));
    }
    if let Some(v) = interpreter {
        mapping.insert(Value::String("interpreter".into()), Value::String(v.into()));
    }
    if let Some(v) = script {
        mapping.insert(Value::String("script".into()), Value::String(v.into()));
    }

    if !add_deps.is_empty() || !rm_deps.is_empty() {
        let mut deps: Vec<String> = mapping
            .get(Value::String("depends_on".into()))
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        for d in add_deps {
            if !deps.contains(d) {
                deps.push(d.clone());
            }
        }
        deps.retain(|d| !rm_deps.contains(d));

        if deps.is_empty() {
            mapping.remove(Value::String("depends_on".into()));
        } else {
            let seq: Vec<Value> = deps.into_iter().map(Value::String).collect();
            mapping.insert(Value::String("depends_on".into()), Value::Sequence(seq));
        }
    }

    // Validate result
    let task: crate::parser::Task = serde_yaml::from_value(doc.clone())
        .map_err(|e| anyhow::anyhow!("Invalid task '{}': {}", name, e))?;
    task.validate().map_err(|e| anyhow::anyhow!("{}", e))?;

    yaml::writer::save_yaml(&path, &doc)?;
    println!("Task '{}' modified", name);
    Ok(())
}

pub fn modify_pipeline(
    name: &str,
    add_tasks: &[String],
    rm_tasks: &[String],
    add_pipelines: &[String],
    rm_pipelines: &[String],
) -> Result<()> {
    let path = format!("bee/pipelines/{}.yml", name);
    let content = file::get_file_content(&String::from(&path))?;
    let mut doc: Value = serde_yaml::from_str(&content)?;

    let mapping = doc
        .as_mapping_mut()
        .ok_or_else(|| anyhow::anyhow!("Invalid pipeline YAML"))?;

    let mut tasks: Vec<String> = mapping
        .get(Value::String("tasks".into()))
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    for t in add_tasks {
        if !tasks.contains(t) {
            tasks.push(t.clone());
        }
    }
    tasks.retain(|t| !rm_tasks.contains(t));

    let seq: Vec<Value> = tasks.into_iter().map(Value::String).collect();
    mapping.insert(Value::String("tasks".into()), Value::Sequence(seq));

    if !add_pipelines.is_empty() || !rm_pipelines.is_empty() {
        let mut pipelines: Vec<String> = mapping
            .get(Value::String("pipelines".into()))
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        for p in add_pipelines {
            if !pipelines.contains(p) {
                pipelines.push(p.clone());
            }
        }
        pipelines.retain(|p| !rm_pipelines.contains(p));

        if pipelines.is_empty() {
            mapping.remove(Value::String("pipelines".into()));
        } else {
            let seq: Vec<Value> = pipelines.into_iter().map(Value::String).collect();
            mapping.insert(Value::String("pipelines".into()), Value::Sequence(seq));
        }
    }

    yaml::writer::save_yaml(&path, &doc)?;
    println!("Pipeline '{}' modified", name);
    Ok(())
}

fn trigger_mapping(trigger_type: &str) -> serde_yaml::Mapping {
    let mut mapping = serde_yaml::Mapping::new();
    mapping.insert(
        Value::String("type".into()),
        Value::String(trigger_type.into()),
    );
    mapping
}

fn required(params: &mut std::collections::HashMap<String, String>, key: &str) -> Result<String> {
    let value = params
        .remove(key)
        .ok_or_else(|| anyhow::anyhow!("--trigger-param {}=... is required", key))?;
    if value.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "--trigger-param {} cannot be empty",
            key
        ));
    }
    Ok(value)
}

fn csv_param(params: &mut std::collections::HashMap<String, String>, key: &str) -> Result<Value> {
    let value = required(params, key)?;
    let items: Vec<Value> = value
        .split(',')
        .map(|item| Value::String(item.trim().to_string()))
        .filter(|item| !matches!(item, Value::String(s) if s.is_empty()))
        .collect();
    if items.is_empty() {
        return Err(anyhow::anyhow!("--trigger-param {} cannot be empty", key));
    }
    Ok(Value::Sequence(items))
}

fn build_trigger(trigger_type: &str, trigger_params: &[String]) -> Result<Value> {
    let mut params: std::collections::HashMap<String, String> = trigger_params
        .iter()
        .map(|param| {
            let mut parts = param.splitn(2, '=');
            let key = parts.next().unwrap_or_default().to_string();
            let value = parts.next().unwrap_or_default().to_string();
            (key, value)
        })
        .collect();

    let mut mapping = trigger_mapping(trigger_type);
    match trigger_type {
        "manual" | "always" => {}
        "file_change" | "git_changed" | "checksum_changed" => {
            let paths = csv_param(&mut params, "paths")?;
            mapping.insert(Value::String("paths".into()), paths);
        }
        "task_completed" | "task_failed" | "dependency_changed"
        | "parent_task_completed" | "parent_task_failed" => {
            let task = required(&mut params, "task")?;
            mapping.insert(Value::String("task".into()), Value::String(task));
        }
        "env_changed" => {
            let vars = csv_param(&mut params, "vars")?;
            mapping.insert(Value::String("vars".into()), vars);
        }
        "dependency_missing" => {
            let path = required(&mut params, "path")?;
            mapping.insert(Value::String("path".into()), Value::String(path));
        }
        "schedule" => {
            let cron = required(&mut params, "cron")?;
            crate::schedule::validate_expression(&cron)?;
            mapping.insert(Value::String("cron".into()), Value::String(cron));
        }
        "git_tag" => {
            if let Some(pattern) = params.remove("pattern") {
                mapping.insert(Value::String("pattern".into()), Value::String(pattern));
            }
        }
        "subtask_completed" | "subtask_failed" | "subtask_changed" => {
            let subsystem = required(&mut params, "subsystem")?;
            let task = required(&mut params, "task")?;
            mapping.insert(Value::String("subsystem".into()), Value::String(subsystem));
            mapping.insert(Value::String("task".into()), Value::String(task));
        }
        "subsystem_changed" => {
            let subsystem = required(&mut params, "subsystem")?;
            mapping.insert(Value::String("subsystem".into()), Value::String(subsystem));
        }
        other => return Err(anyhow::anyhow!("unknown trigger type: {}", other)),
    }

    if let Some(key) = params.keys().next() {
        return Err(anyhow::anyhow!(
            "unknown --trigger-param '{}' for trigger '{}'",
            key,
            trigger_type
        ));
    }

    let value = Value::Mapping(mapping);
    serde_yaml::from_value::<crate::parser::RuleTrigger>(value.clone())
        .map_err(|e| anyhow::anyhow!("invalid trigger: {}", e))?;
    Ok(value)
}

pub fn modify_rule(
    name: &str,
    task: Option<&str>,
    trigger_type: Option<&str>,
    trigger_params: &[String],
) -> Result<()> {
    let path = format!("bee/rules/{}.yml", name);
    let content = file::get_file_content(&String::from(&path))?;
    let mut doc: Value = serde_yaml::from_str(&content)?;

    if let Some(t) = task {
        doc.as_mapping_mut()
            .ok_or_else(|| anyhow::anyhow!("Invalid rule YAML"))?
            .insert(Value::String("task".into()), Value::String(t.into()));
    }

    if let Some(tt) = trigger_type {
        let trigger = build_trigger(tt, trigger_params)?;
        let mapping = doc
            .as_mapping_mut()
            .ok_or_else(|| anyhow::anyhow!("Invalid rule YAML"))?;
        match mapping.get_mut(Value::String("triggers".into())) {
            Some(Value::Sequence(seq)) => seq.push(trigger),
            Some(_) => {
                return Err(anyhow::anyhow!(
                    "rule '{}' has a malformed 'triggers' field",
                    name
                ))
            }
            None => {
                mapping.insert(
                    Value::String("triggers".into()),
                    Value::Sequence(vec![trigger]),
                );
            }
        }
    }

    // Validate the complete rule before writing it, so CLI edits can never
    // produce a rule file that is silently ignored at run time.
    let yaml_text = serde_yaml::to_string(&doc)?;
    crate::parser::reader::config::read_rules_from_string(&yaml_text)
        .map_err(|e| anyhow::anyhow!("refusing to write invalid rule: {}", e))?;
    yaml::writer::save_yaml(&path, &doc)?;
    println!("Rule '{}' modified", name);
    Ok(())
}

pub fn modify_dep(
    name: &str,
    dep_type: Option<&str>,
    source: Option<&str>,
    version: Option<&str>,
    command: Option<&str>,
    install_script: Option<&str>,
    path: Option<&str>,
) -> Result<()> {
    let deps_path = "bee/system/deps.yml";
    let content = file::get_file_content(&String::from(deps_path))?;
    let mut doc: Value = serde_yaml::from_str(&content)?;

    let deps = doc
        .get_mut("dependencies")
        .and_then(|v| v.as_sequence_mut())
        .ok_or_else(|| anyhow::anyhow!("No dependencies in deps.yml"))?;

    let dep = deps
        .iter_mut()
        .find(|d| d.get("name").and_then(|v| v.as_str()) == Some(name))
        .ok_or_else(|| anyhow::anyhow!("Dependency '{}' not found", name))?;

    let mapping = dep
        .as_mapping_mut()
        .ok_or_else(|| anyhow::anyhow!("Invalid dependency entry"))?;

    if let Some(t) = dep_type {
        mapping.insert(Value::String("type".into()), Value::String(t.into()));
    }
    if let Some(s) = source {
        mapping.insert(Value::String("source".into()), Value::String(s.into()));
    }
    if let Some(v) = version {
        mapping.insert(Value::String("version".into()), Value::String(v.into()));
    }
    if let Some(c) = command {
        mapping.insert(Value::String("command".into()), Value::String(c.into()));
    }
    if let Some(s) = install_script {
        mapping.insert(Value::String("install_script".into()), Value::Bool(true));
        let script_path = format!("bee/scripts/install/{}.sh", name);
        std::fs::create_dir_all("bee/scripts/install")?;
        file::write_file_content(&script_path, s)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    if let Some(p) = path {
        mapping.insert(Value::String("path".into()), Value::String(p.to_owned()));
    }

    yaml::writer::save_yaml(deps_path, &doc)?;

    let deps_hash = hash::hash_string(&file::get_file_content(&String::from(deps_path))?);
    hash::write_hash_file("bee/system/hash/deps", &deps_hash)?;

    println!("Dependency '{}' modified", name);
    Ok(())
}
