use std::process::Command;
use crate::parser::{Rule, RuleTrigger};
use crate::parser::reader::config as reader;

pub fn should_run(task_name: &str) -> bool {
    let rules = reader::read_rules().unwrap_or_else(|_| vec![]);

    let task_rules: Vec<&Rule> = rules.iter()
        .filter(|r| r.task == task_name)
        .collect();

    if task_rules.is_empty() {
        return true;
    }

    for rule in &task_rules {
        if evaluate_triggers(&rule.triggers, task_name) {
            return true;
        }
    }

    false
}

fn evaluate_triggers(triggers: &[RuleTrigger], task_name: &str) -> bool {
    if triggers.is_empty() {
        return true;
    }

    for trigger in triggers {
        if evaluate_trigger(trigger, task_name) {
            return true;
        }
    }

    false
}

fn evaluate_trigger(trigger: &RuleTrigger, task_name: &str) -> bool {
    match trigger {
        RuleTrigger::Manual => false,

        RuleTrigger::Always => true,

        RuleTrigger::FileChange { paths } => {
            check_file_changes(paths)
        }

        RuleTrigger::GitChanged { paths } => {
            check_git_changes(paths)
        }

        RuleTrigger::TaskCompleted { task } => {
            check_task_completed(task)
        }

        RuleTrigger::TaskFailed { task } => {
            check_task_failed(task)
        }

        RuleTrigger::DependencyChanged { task } => {
            check_task_completed(task)
        }

        RuleTrigger::DependencyMissing { path } => {
            !std::path::Path::new(path).exists()
        }

        RuleTrigger::EnvChanged { vars } => {
            check_env_changed(vars, task_name)
        }

        RuleTrigger::Schedule { cron: _ } => {
            true
        }

        RuleTrigger::GitTag { pattern: _ } => {
            check_git_tag()
        }

        RuleTrigger::ChecksumChanged { paths } => {
            check_checksum_changed(paths, task_name)
        }
    }
}

fn check_file_changes(paths: &[String]) -> bool {
    let last_hash = get_trigger_state("file_change");
    let current_hash = compute_files_hash(paths);

    if last_hash.is_none() {
        save_trigger_state("file_change", &current_hash);
        return true;
    }

    let changed = last_hash.unwrap() != current_hash;
    if changed {
        save_trigger_state("file_change", &current_hash);
    }
    changed
}

fn check_git_changes(paths: &[String]) -> bool {
    let output = Command::new("git")
        .args(["diff", "--name-only", "HEAD~1"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let changed_files = String::from_utf8_lossy(&out.stdout);
            for changed_file in changed_files.lines() {
                for pattern in paths {
                    if glob_match(pattern, changed_file) {
                        return true;
                    }
                }
            }
            false
        }
        _ => true
    }
}

fn check_task_completed(task: &str) -> bool {
    let state = get_trigger_state(&format!("task_{}", task));
    state.map(|s| s == "completed").unwrap_or(false)
}

fn check_task_failed(task: &str) -> bool {
    let state = get_trigger_state(&format!("task_{}", task));
    state.map(|s| s == "failed").unwrap_or(false)
}

fn check_env_changed(vars: &[String], task_name: &str) -> bool {
    let state_key = format!("env_{}", task_name);
    let last_state = get_trigger_state(&state_key);

    let current_state: Vec<String> = vars.iter()
        .filter_map(|var| std::env::var(var).ok())
        .collect();

    let current_hash = crate::hash::hash_string(&current_state.join("|"));

    if let Some(last) = last_state {
        let changed = last != current_hash;
        if changed {
            save_trigger_state(&state_key, &current_hash);
        }
        changed
    } else {
        save_trigger_state(&state_key, &current_hash);
        true
    }
}

fn check_git_tag() -> bool {
    let output = Command::new("git")
        .args(["describe", "--tags", "--exact-match", "HEAD"])
        .output();

    output.map(|o| o.status.success()).unwrap_or(false)
}

fn check_checksum_changed(paths: &[String], task_name: &str) -> bool {
    let state_key = format!("checksum_{}", task_name);
    let last_hash = get_trigger_state(&state_key);

    let mut combined = String::new();
    for path in paths {
        if let Ok(h) = crate::hash::hash_file(path) {
            combined.push_str(&h);
        }
    }

    let current_hash = crate::hash::hash_string(&combined);

    if let Some(last) = last_hash {
        let changed = last != current_hash;
        if changed {
            save_trigger_state(&state_key, &current_hash);
        }
        changed
    } else {
        save_trigger_state(&state_key, &current_hash);
        true
    }
}

fn compute_files_hash(paths: &[String]) -> String {
    let mut combined = String::new();
    for pattern in paths {
        if let Ok(entries) = glob::glob(pattern) {
            for entry in entries.flatten() {
                if let Some(s) = entry.to_str() {
                    if let Ok(h) = crate::hash::hash_file(s) {
                        combined.push_str(&h);
                    }
                }
            }
        }
    }
    crate::hash::hash_string(&combined)
}

fn glob_match(pattern: &str, path: &str) -> bool {
    glob::glob(pattern)
        .ok()
        .map(|mut entries| entries.any(|e| e.map(|p| p == std::path::Path::new(path)).unwrap_or(false)))
        .unwrap_or(false)
}

fn get_trigger_state(key: &str) -> Option<String> {
    let path = format!("bee/cache/triggers/{}", key);
    std::fs::read_to_string(path).ok()
}

fn save_trigger_state(key: &str, value: &str) {
    let _ = std::fs::create_dir_all("bee/cache/triggers");
    let _ = std::fs::write(format!("bee/cache/triggers/{}", key), value);
}

pub fn mark_task_state(task_name: &str, state: &str) {
    save_trigger_state(&format!("task_{}", task_name), state);
}

pub fn run_task(task: crate::parser::Task) -> anyhow::Result<()> {
    if !should_run(&task.name) {
        println!("  - {}: skipped (no triggers fired)", task.name);
        return Ok(());
    }

    println!("  - {}: running", task.name);

    let result = run_command(&task.run);

    match result {
        Ok((stdout, stderr)) => {
            print!("{}", stdout);
            eprint!("{}", stderr);
            mark_task_state(&task.name, "completed");
        }
        Err(e) => {
            mark_task_state(&task.name, "failed");
            return Err(e);
        }
    }

    Ok(())
}

pub fn run_command(command: &str) -> anyhow::Result<(String, String)> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Command failed: {}", stderr));
    }

    Ok((String::from_utf8_lossy(&output.stdout).to_string(), String::from_utf8_lossy(&output.stderr).to_string()))
}
