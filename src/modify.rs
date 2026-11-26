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
        let mapping = doc
            .as_mapping_mut()
            .ok_or_else(|| anyhow::anyhow!("Invalid rule YAML"))?;

        let mut params: std::collections::HashMap<String, String> = trigger_params
            .iter()
            .filter_map(|p| {
                let mut parts = p.splitn(2, '=');
                let key = parts.next()?.to_string();
                let val = parts.next().unwrap_or("").to_string();
                Some((key, val))
            })
            .collect();

        let trigger = match tt {
            "manual" => Value::Mapping(serde_yaml::Mapping::new()),
            "always" => {
                let mut m = serde_yaml::Mapping::new();
                m.insert(Value::String("type".into()), Value::String("always".into()));
                Value::Mapping(m)
            }
            "file_change" | "git_changed" | "checksum_changed" => {
                let paths: Vec<Value> = params
                    .remove("paths")
                    .unwrap_or_default()
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| Value::String(s.trim().into()))
                    .collect();
                let mut m = serde_yaml::Mapping::new();
                m.insert(Value::String("type".into()), Value::String(tt.into()));
                m.insert(Value::String("paths".into()), Value::Sequence(paths));
                Value::Mapping(m)
            }
            "task_completed" | "task_failed" | "dependency_changed" => {
                let task_name = params.remove("task").unwrap_or_default();
                let mut m = serde_yaml::Mapping::new();
                m.insert(Value::String("type".into()), Value::String(tt.into()));
                m.insert(Value::String("task".into()), Value::String(task_name));
                Value::Mapping(m)
            }
            "env_changed" => {
                let vars: Vec<Value> = params
                    .remove("vars")
                    .unwrap_or_default()
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| Value::String(s.trim().into()))
                    .collect();
                let mut m = serde_yaml::Mapping::new();
                m.insert(Value::String("type".into()), Value::String(tt.into()));
                m.insert(Value::String("vars".into()), Value::Sequence(vars));
                Value::Mapping(m)
            }
            "dependency_missing" => {
                let p = params.remove("path").unwrap_or_default();
                let mut m = serde_yaml::Mapping::new();
                m.insert(Value::String("type".into()), Value::String(tt.into()));
                m.insert(Value::String("path".into()), Value::String(p));
                Value::Mapping(m)
            }
            "schedule" => {
                let cron = params.remove("cron").unwrap_or_default();
                let mut m = serde_yaml::Mapping::new();
                m.insert(Value::String("type".into()), Value::String(tt.into()));
                m.insert(Value::String("cron".into()), Value::String(cron));
                Value::Mapping(m)
            }
            "git_tag" => {
                let pattern = params.remove("pattern");
                let mut m = serde_yaml::Mapping::new();
                m.insert(Value::String("type".into()), Value::String(tt.into()));
                if let Some(p) = pattern {
                    m.insert(Value::String("pattern".into()), Value::String(p));
                }
                Value::Mapping(m)
            }
            "subtask_completed" | "subtask_failed" | "subtask_changed" => {
                let subsystem = params.remove("subsystem").unwrap_or_default();
                let task_name = params.remove("task").unwrap_or_default();
                let mut m = serde_yaml::Mapping::new();
                m.insert(Value::String("type".into()), Value::String(tt.into()));
                m.insert(Value::String("subsystem".into()), Value::String(subsystem));
                m.insert(Value::String("task".into()), Value::String(task_name));
                Value::Mapping(m)
            }
            "parent_task_completed" | "parent_task_failed" => {
                let task_name = params.remove("task").unwrap_or_default();
                let mut m = serde_yaml::Mapping::new();
                m.insert(Value::String("type".into()), Value::String(tt.into()));
                m.insert(Value::String("task".into()), Value::String(task_name));
                Value::Mapping(m)
            }
            "subsystem_changed" => {
                let subsystem = params.remove("subsystem").unwrap_or_default();
                let mut m = serde_yaml::Mapping::new();
                m.insert(Value::String("type".into()), Value::String(tt.into()));
                m.insert(Value::String("subsystem".into()), Value::String(subsystem));
                Value::Mapping(m)
            }
            _ => return Err(anyhow::anyhow!("Unknown trigger type: {}", tt)),
        };

        if let Some(triggers) = mapping.get_mut(Value::String("triggers".into())) {
            if let Some(seq) = triggers.as_sequence_mut() {
                seq.push(trigger);
            }
        } else {
            mapping.insert(
                Value::String("triggers".into()),
                Value::Sequence(vec![trigger]),
            );
        }
    }

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
