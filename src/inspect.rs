//! Read-only inspection commands: validate/doctor, logs, show and JSON output
//! for list/status.

use crate::parser::reader::config as reader;
use crate::{file, hash, safety, ui};
use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Report {
    pub problems: Vec<String>,
    pub checked: usize,
}

/// Validate the whole configuration without stopping at the first problem.
pub fn validate() -> Result<Report> {
    let mut problems = Vec::new();
    let mut checked = 0usize;

    let config = file::get_file_content("bee/system/config.yml")?;
    let expected_hash = hash::hash_string(&config);
    match fs::read_to_string("bee/system/hash/config") {
        Ok(stored) if stored.trim() == expected_hash => {}
        Ok(_) => problems.push(
            "bee/system/hash/config does not match config.yml; run 'bee docs update' or restore from backup".to_string(),
        ),
        Err(err) => problems.push(format!("cannot read bee/system/hash/config: {}", err)),
    }

    let tasks = reader::read_tasks_config()?;
    let task_names: HashSet<String> = tasks.iter().map(|t| t.name.clone()).collect();
    for task in &tasks {
        checked += 1;
        let path = definition_path("task", &task.name)?;
        match fs::read_to_string(&path) {
            Ok(content) => {
                if let Err(err) = reader::read_task_from_string(&content, &task.name) {
                    problems.push(format!("bee/tasks/{}.yml: {}", task.name, err));
                }
            }
            Err(err) => problems.push(format!(
                "bee/tasks/{}.yml: cannot read file: {}",
                task.name, err
            )),
        }
    }

    let pipelines = reader::read_pipelines_config()?;
    let pipeline_names: HashSet<String> = pipelines.iter().map(|p| p.name.clone()).collect();
    for pipeline in &pipelines {
        checked += 1;
        let path = definition_path("pipeline", &pipeline.name)?;
        match fs::read_to_string(&path) {
            Ok(content) => match reader::read_pipeline_from_string(&content) {
                Ok(parsed) => {
                    for task in &parsed.tasks {
                        if !task_names.contains(task) {
                            problems.push(format!(
                                "bee/pipelines/{}.yml: task '{}' is not registered",
                                pipeline.name, task
                            ));
                        }
                    }
                    for sub in &parsed.pipelines {
                        if !pipeline_names.contains(sub) {
                            problems.push(format!(
                                "bee/pipelines/{}.yml: subpipeline '{}' is not registered",
                                pipeline.name, sub
                            ));
                        }
                    }
                    if let Err(err) = crate::run::dag::build_dag(parsed) {
                        problems.push(format!("bee/pipelines/{}.yml: {}", pipeline.name, err));
                    }
                }
                Err(err) => problems.push(format!("bee/pipelines/{}.yml: {}", pipeline.name, err)),
            },
            Err(err) => problems.push(format!(
                "bee/pipelines/{}.yml: cannot read file: {}",
                pipeline.name, err
            )),
        }
    }

    let rules = reader::read_rules_config()?;
    for rule in &rules {
        checked += 1;
        let path = definition_path("rule", &rule.name)?;
        match fs::read_to_string(&path) {
            Ok(content) => match reader::read_rules_from_string(&content) {
                Ok(parsed) => {
                    if !task_names.contains(&parsed.task) {
                        problems.push(format!(
                            "bee/rules/{}.yml: task '{}' is not registered",
                            rule.name, parsed.task
                        ));
                    }
                }
                Err(err) => problems.push(format!("bee/rules/{}.yml: {}", rule.name, err)),
            },
            Err(err) => problems.push(format!(
                "bee/rules/{}.yml: cannot read file: {}",
                rule.name, err
            )),
        }
    }

    Ok(Report { problems, checked })
}

pub fn validate_command() -> Result<()> {
    let report = validate()?;
    if ui::json_mode() {
        let mut problems = json::Array::new();
        for problem in &report.problems {
            problems.push(problem.as_str().into());
        }
        ui::print_json(&json::object! {
            "ok" => report.problems.is_empty(),
            "checked" => report.checked,
            "problems" => problems,
        });
    } else if report.problems.is_empty() {
        println!(
            "{}",
            ui::green(&format!(
                "OK: configuration is valid ({} object(s) checked)",
                report.checked
            ))
        );
    } else {
        for problem in &report.problems {
            eprintln!("{}", ui::red(&format!("  - {}", problem)));
        }
    }

    if report.problems.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(
            "configuration validation failed with {} problem(s)",
            report.problems.len()
        ))
    }
}

fn definition_path(kind: &str, name: &str) -> Result<PathBuf> {
    let (dir, label) = match kind {
        "task" => ("bee/tasks", "task"),
        "pipeline" => ("bee/pipelines", "pipeline"),
        "rule" => ("bee/rules", "rule"),
        other => return Err(anyhow!("unknown definition kind '{}'", other)),
    };
    Ok(safety::safe_path(dir, label, name)?.with_extension("yml"))
}

pub fn logs_list() -> Result<Vec<String>> {
    let mut names = Vec::new();
    let dir = PathBuf::from("bee/logs");
    if !dir.exists() {
        return Ok(names);
    }
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(task) = name.strip_suffix(".log") {
            names.push(task.to_string());
        }
    }
    names.sort();
    Ok(names)
}

pub fn logs_list_command() -> Result<()> {
    let names = logs_list()?;
    if ui::json_mode() {
        let mut logs = json::Array::new();
        for name in &names {
            let log_path = PathBuf::from("bee/logs").join(format!("{}.log", name));
            logs.push(json::object! {
                "task" => name.as_str(),
                "path" => log_path.to_string_lossy().as_ref(),
                "bytes" => fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0),
            });
        }
        ui::print_json(&json::object! { "logs" => logs });
        return Ok(());
    }
    if names.is_empty() {
        println!("No task logs yet.");
    } else {
        println!("Task logs:");
        for name in &names {
            println!("  {} -> bee/logs/{}.log", name, name);
        }
    }
    Ok(())
}

pub fn logs_task_command(name: &str) -> Result<()> {
    safety::validate_name("task", name)?;
    let path = PathBuf::from("bee/logs").join(format!("{}.log", name));
    let content = fs::read_to_string(&path).map_err(|_| {
        anyhow!(
            "no log for task '{}' (file '{}' not found)",
            name,
            path.display()
        )
    })?;
    if ui::json_mode() {
        ui::print_json(&json::object! {
            "task" => name,
            "path" => path.to_string_lossy().as_ref(),
            "content" => content.as_str(),
        });
    } else {
        print!("{}", content);
        if !content.ends_with('\n') {
            println!();
        }
    }
    Ok(())
}

pub fn show_command(kind: &str, name: &str) -> Result<()> {
    let path = definition_path(kind, name)?;
    let content =
        fs::read_to_string(&path).map_err(|_| anyhow!("{} '{}' not found", kind, name))?;
    // Parsing here means `bee show` never presents an invalid definition as valid.
    match kind {
        "task" => {
            reader::read_task_from_string(&content, name)?;
        }
        "pipeline" => {
            let parsed = reader::read_pipeline_from_string(&content)?;
            if parsed.name != name {
                return Err(anyhow!("pipeline name in file does not match '{}'", name));
            }
        }
        "rule" => {
            let parsed = reader::read_rules_from_string(&content)?;
            if parsed.name != name {
                return Err(anyhow!("rule name in file does not match '{}'", name));
            }
        }
        _ => return Err(anyhow!("unknown definition kind '{}'", kind)),
    }
    if ui::json_mode() {
        ui::print_json(&json::object! {
            "kind" => kind,
            "name" => name,
            "path" => path.to_string_lossy().as_ref(),
            "definition" => content.as_str(),
        });
    } else {
        print!("{}", content);
        if !content.ends_with('\n') {
            println!();
        }
    }
    Ok(())
}

pub fn list_json() -> Result<json::JsonValue> {
    let mut pipelines = json::Array::new();
    for item in reader::read_pipelines_config()? {
        pipelines.push(item.name.as_str().into());
    }
    let mut tasks = json::Array::new();
    for item in reader::read_tasks_config()? {
        tasks.push(item.name.as_str().into());
    }
    let mut rules = json::Array::new();
    for item in reader::read_rules_config()? {
        rules.push(item.name.as_str().into());
    }
    Ok(json::object! {
        "pipelines" => pipelines,
        "tasks" => tasks,
        "rules" => rules,
    })
}

pub fn status_json() -> Result<json::JsonValue> {
    let pipelines = reader::read_pipelines()?;
    let tasks = reader::read_tasks_config()?;
    let rules = reader::read_rules_config()?;
    let cached: Vec<&str> = tasks
        .iter()
        .filter(|task| {
            fs::read_to_string(crate::run::cache::last_run_path(&task.name))
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
        })
        .map(|task| task.name.as_str())
        .collect();
    let cached_names: json::Array = cached
        .iter()
        .map(|name| json::JsonValue::from(*name))
        .collect();
    Ok(json::object! {
        "pipelines" => pipelines.len(),
        "tasks" => tasks.len(),
        "rules" => rules.len(),
        "cached_tasks" => cached.len(),
        "cached" => cached_names,
    })
}
