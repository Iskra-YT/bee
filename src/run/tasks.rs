use std::process::Command;
use crate::parser::{Rule, RuleTrigger};
use crate::parser::reader::config as reader;
use crate::run::cache;

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
            check_file_changes(paths, task_name)
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

        RuleTrigger::SubtaskCompleted { subsystem, task } => {
            check_subtask_state(subsystem, task, "completed")
        }

        RuleTrigger::SubtaskFailed { subsystem, task } => {
            check_subtask_state(subsystem, task, "failed")
        }

        RuleTrigger::SubtaskChanged { subsystem, task } => {
            check_subtask_changed(subsystem, task)
        }

        RuleTrigger::ParentTaskCompleted { task } => {
            check_parent_task_state(task, "completed")
        }

        RuleTrigger::ParentTaskFailed { task } => {
            check_parent_task_state(task, "failed")
        }

        RuleTrigger::SubsystemChanged { subsystem } => {
            check_subsystem_changed(subsystem)
        }
    }
}

fn check_file_changes(paths: &[String], task_name: &str) -> bool {
    let state_key = format!("file_change_{}_{}", task_name, crate::hash::hash_string(&paths.join("|")));
    let last_hash = get_trigger_state(&state_key);
    let current_hash = compute_files_hash(paths);

    if last_hash.is_none() {
        save_trigger_state(&state_key, &current_hash);
        return true;
    }

    let changed = last_hash.unwrap() != current_hash;
    if changed {
        save_trigger_state(&state_key, &current_hash);
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
    let state_key = format!("env_{}_{}", task_name, crate::hash::hash_string(&vars.join("|")));
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
    let state_key = format!("checksum_{}_{}", task_name, crate::hash::hash_string(&paths.join("|")));
    let last_hash = get_trigger_state(&state_key);

    let current_hash = compute_files_hash(paths);

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

fn check_subtask_state(subsystem: &str, task: &str, expected: &str) -> bool {
    let Some(dir) = crate::sub::resolve_subsystem_dir(subsystem) else {
        return false;
    };
    let state = cache::get_state_at(&dir.to_string_lossy(), &format!("task_{}", task));
    state.map(|s| s == expected).unwrap_or(false)
}

fn check_parent_task_state(task: &str, expected: &str) -> bool {
    let Some(dir) = crate::file::resolve_parent_dir() else {
        return false;
    };
    let state = cache::get_state_at(&dir.to_string_lossy(), &format!("task_{}", task));
    state.map(|s| s == expected).unwrap_or(false)
}

fn check_subtask_changed(subsystem: &str, task: &str) -> bool {
    let state_key = format!("subtask_{}_{}", subsystem, task);
    let last_state = get_trigger_state(&state_key);
    let Some(dir) = crate::sub::resolve_subsystem_dir(subsystem) else {
        return false;
    };
    let current = cache::get_state_at(&dir.to_string_lossy(), &format!("task_{}", task))
        .unwrap_or_else(|| String::from("none"));

    if let Some(last) = last_state {
        let changed = last != current;
        if changed {
            save_trigger_state(&state_key, &current);
        }
        changed
    } else {
        save_trigger_state(&state_key, &current);
        true
    }
}

fn check_subsystem_changed(subsystem: &str) -> bool {
    let state_key = format!("subsystem_state_{}", subsystem);
    let last_state = get_trigger_state(&state_key);
    let Some(dir) = crate::sub::resolve_subsystem_dir(subsystem) else {
        return false;
    };
    let current = compute_subsystem_state_hash(&dir);

    if let Some(last) = last_state {
        let changed = last != current;
        if changed {
            save_trigger_state(&state_key, &current);
        }
        changed
    } else {
        save_trigger_state(&state_key, &current);
        true
    }
}

fn compute_subsystem_state_hash(dir: &std::path::Path) -> String {
    let mut entries: Vec<String> = vec![];
    if let Ok(triggers) = std::fs::read_dir(dir.join("bee/cache/triggers")) {
        for entry in triggers.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("task_") {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    entries.push(format!("{}={}", name, content.trim()));
                }
            }
        }
    }
    entries.sort();
    crate::hash::hash_string(&entries.join("|"))
}

fn get_trigger_state(key: &str) -> Option<String> {
    cache::get_state(key)
}

fn save_trigger_state(key: &str, value: &str) {
    let _ = cache::save_state(key, value);
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
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);

    if let Some(owner) = crate::sub::deps_owner_dir() {
        cmd.env("BEE_DEPS_PATH", owner.join("bee/deps"));
    }

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let code = output.status.code().unwrap_or(-1);
        let msg = if stderr.trim().is_empty() {
            format!("exit code {}", code)
        } else {
            format!("exit code {}: {}", code, stderr.trim())
        };
        return Err(anyhow::anyhow!("{}", msg));
    }

    Ok((String::from_utf8_lossy(&output.stdout).to_string(), String::from_utf8_lossy(&output.stderr).to_string()))
}
