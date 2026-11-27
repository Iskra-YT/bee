use crate::parser::reader::config as reader;
use crate::parser::{Rule, RuleTrigger};
use crate::run::cache;
use anyhow::{anyhow, Result};
use std::fmt;
use std::process::Command;

/// Failure of a task process, preserving the real exit code for the CLI.
#[derive(Debug, Clone)]
pub struct TaskFailure {
    pub task: String,
    pub exit_code: i32,
    pub message: String,
}

impl fmt::Display for TaskFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TaskFailure {}

/// Extract a task exit code from any error in the chain.
pub fn failure_exit_code(error: &anyhow::Error) -> Option<i32> {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<TaskFailure>())
        .map(|failure| failure.exit_code)
}

pub fn should_run(task_name: &str, force: bool) -> Result<bool> {
    // Read rules even when forced: a broken rule file must never be hidden.
    let rules = reader::read_rules()?;
    if force {
        return Ok(true);
    }

    let task_rules: Vec<&Rule> = rules.iter().filter(|r| r.task == task_name).collect();
    if task_rules.is_empty() {
        return Ok(true);
    }
    for rule in &task_rules {
        if evaluate_triggers(&rule.triggers, task_name) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether a scheduled trigger of any rule for this task is due at `now`.
pub fn schedule_due(
    task_name: &str,
    now: &chrono::DateTime<chrono::Local>,
) -> Result<bool> {
    let rules = reader::read_rules()?;
    for rule in rules.iter().filter(|r| r.task == task_name) {
        for trigger in &rule.triggers {
            if let RuleTrigger::Schedule { cron } = trigger {
                if crate::schedule::matches_expression(cron, now)? {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn evaluate_triggers(triggers: &[RuleTrigger], task_name: &str) -> bool {
    if triggers.is_empty() {
        return true;
    }
    triggers
        .iter()
        .any(|trigger| evaluate_trigger(trigger, task_name))
}

fn evaluate_trigger(trigger: &RuleTrigger, task_name: &str) -> bool {
    match trigger {
        RuleTrigger::Manual => false,
        RuleTrigger::Always => true,
        // Cron is evaluated by `bee schedule run`, not by one-shot commands.
        RuleTrigger::Schedule { .. } => false,
        RuleTrigger::FileChange { paths } => check_file_changes(paths, task_name),
        RuleTrigger::GitChanged { paths } => check_git_changes(paths, task_name),
        RuleTrigger::TaskCompleted { task } => check_task_completed(task),
        RuleTrigger::TaskFailed { task } => check_task_failed(task),
        RuleTrigger::DependencyChanged { task } => check_task_completed(task),
        RuleTrigger::DependencyMissing { path } => !std::path::Path::new(path).exists(),
        RuleTrigger::EnvChanged { vars } => check_env_changed(vars, task_name),
        RuleTrigger::GitTag { pattern } => check_git_tag(pattern.as_deref(), task_name),
        RuleTrigger::ChecksumChanged { paths } => check_checksum_changed(paths, task_name),
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
        RuleTrigger::SubsystemChanged { subsystem } => check_subsystem_changed(subsystem),
    }
}

fn state_key(kind: &str, task_name: &str, config: &[String]) -> String {
    format!(
        "{}_{}_{}",
        kind,
        task_name,
        crate::hash::hash_string(&config.join("\x1f"))
    )
}

fn save_pending(key: &str, value: &str) {
    if let Err(err) = cache::save_state(key, value) {
        eprintln!("Warning: could not save trigger state: {}", err);
    }
}

fn check_file_changes(paths: &[String], task_name: &str) -> bool {
    let key = state_key("file_change", task_name, paths);
    let current = compute_files_hash(paths);
    let Some(last) = cache::get_state(&key) else {
        // Establish a baseline without firing: nothing has changed yet.
        save_pending(&key, &current);
        return false;
    };
    let changed = last.trim() != current;
    if changed {
        save_pending(&key, &current);
    }
    changed
}

fn git_output(args: &[&str]) -> Option<String> {
    match Command::new("git").args(args).output() {
        Ok(out) if out.status.success() => {
            Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
        }
        _ => None,
    }
}

fn check_git_changes(paths: &[String], task_name: &str) -> bool {
    let key = state_key("git_changed", task_name, paths);
    let Some(head) = git_output(&["rev-parse", "HEAD"]) else {
        eprintln!("Warning: git_changed trigger needs a git repository with a commit");
        return false;
    };

    let Some(last) = cache::get_state(&key) else {
        save_pending(&key, &head);
        return false;
    };
    if last.trim() == head {
        return false;
    }

    // The commit moved: stage the new baseline and fire only if a path matched.
    save_pending(&key, &head);
    let changed_files = git_output(&["diff", "--name-only", "HEAD~1", "HEAD"]).unwrap_or_default();
    changed_files
        .lines()
        .any(|file| paths.iter().any(|pattern| glob_match(pattern, file)))
}

fn check_task_completed(task: &str) -> bool {
    cache::get_state(&format!("task_{}", task))
        .map(|s| s.trim() == "completed")
        .unwrap_or(false)
}

fn check_task_failed(task: &str) -> bool {
    cache::get_state(&format!("task_{}", task))
        .map(|s| s.trim() == "failed")
        .unwrap_or(false)
}

fn check_env_changed(vars: &[String], task_name: &str) -> bool {
    let key = state_key("env_changed", task_name, vars);
    let mut current: Vec<String> = vars
        .iter()
        .map(|var| match std::env::var(var) {
            Ok(value) => format!("{}={}", var, value),
            Err(_) => format!("{}=<unset>", var),
        })
        .collect();
    current.sort();
    let current_hash = crate::hash::hash_string(&current.join("\x1f"));

    let Some(last) = cache::get_state(&key) else {
        save_pending(&key, &current_hash);
        return false;
    };
    let changed = last.trim() != current_hash;
    if changed {
        save_pending(&key, &current_hash);
    }
    changed
}

fn check_git_tag(pattern: Option<&str>, task_name: &str) -> bool {
    let key = state_key("git_tag", task_name, &[pattern.unwrap_or("*").to_string()]);
    let tag = git_output(&["describe", "--tags", "--exact-match", "HEAD"]).unwrap_or_default();
    let Some(last) = cache::get_state(&key) else {
        save_pending(&key, &tag);
        return false;
    };
    let changed = last.trim() != tag;
    if !changed {
        return false;
    }
    save_pending(&key, &tag);
    let matches = glob::Pattern::new(pattern.unwrap_or("*"))
        .map(|p| p.matches(&tag))
        .unwrap_or(false);
    matches && !tag.is_empty()
}

fn check_checksum_changed(paths: &[String], task_name: &str) -> bool {
    let key = state_key("checksum", task_name, paths);
    let current = compute_files_hash(paths);
    let Some(last) = cache::get_state(&key) else {
        save_pending(&key, &current);
        return false;
    };
    let changed = last.trim() != current;
    if changed {
        save_pending(&key, &current);
    }
    changed
}

fn compute_files_hash(paths: &[String]) -> String {
    let mut entries: Vec<String> = Vec::new();
    for pattern in paths {
        if let Ok(entries_iter) = glob::glob(pattern) {
            for entry in entries_iter.flatten() {
                let Some(path) = entry.to_str() else { continue };
                match crate::hash::hash_file(path) {
                    Ok(hash) => entries.push(format!("{}:{}", path, hash)),
                    Err(err) => {
                        eprintln!("Warning: cannot hash '{}': {}", path, err);
                        entries.push(format!("{}:unreadable", path));
                    }
                }
            }
        }
    }
    entries.sort();
    crate::hash::hash_string(&entries.join("\x1f"))
}

fn glob_match(pattern: &str, path: &str) -> bool {
    glob::Pattern::new(pattern)
        .map(|p| p.matches(path) || p.matches_path(std::path::Path::new(path)))
        .unwrap_or(false)
}

fn check_subtask_state(subsystem: &str, task: &str, expected: &str) -> bool {
    let Some(dir) = crate::subsystem::resolve_subsystem_dir(subsystem) else {
        return false;
    };
    cache::get_state_at(&dir.to_string_lossy(), &format!("task_{}", task))
        .map(|s| s.trim() == expected)
        .unwrap_or(false)
}

fn check_parent_task_state(task: &str, expected: &str) -> bool {
    let Some(dir) = crate::file::resolve_parent_dir() else {
        return false;
    };
    cache::get_state_at(&dir.to_string_lossy(), &format!("task_{}", task))
        .map(|s| s.trim() == expected)
        .unwrap_or(false)
}

fn check_subtask_changed(subsystem: &str, task: &str) -> bool {
    let key = format!("subtask_{}_{}", subsystem, task);
    let Some(dir) = crate::subsystem::resolve_subsystem_dir(subsystem) else {
        return false;
    };
    let current = cache::get_state_at(&dir.to_string_lossy(), &format!("task_{}", task))
        .unwrap_or_else(|| "none".to_string());
    let Some(last) = cache::get_state(&key) else {
        save_pending(&key, &current);
        return false;
    };
    let changed = last.trim() != current;
    if changed {
        save_pending(&key, &current);
    }
    changed
}

fn check_subsystem_changed(subsystem: &str) -> bool {
    let key = format!("subsystem_state_{}", subsystem);
    let Some(dir) = crate::subsystem::resolve_subsystem_dir(subsystem) else {
        return false;
    };
    let current = compute_subsystem_state_hash(&dir);
    let Some(last) = cache::get_state(&key) else {
        save_pending(&key, &current);
        return false;
    };
    let changed = last.trim() != current;
    if changed {
        save_pending(&key, &current);
    }
    changed
}

fn compute_subsystem_state_hash(dir: &std::path::Path) -> String {
    let mut entries: Vec<String> = Vec::new();
    if let Ok(triggers) = std::fs::read_dir(dir.join("bee/cache/triggers")) {
        for entry in triggers.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("task_") || name.starts_with('.') {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                entries.push(format!("{}={}", name, content.trim()));
            }
        }
    }
    entries.sort();
    crate::hash::hash_string(&entries.join("\x1f"))
}

fn mark_task_state(task_name: &str, state: &str) -> Result<()> {
    cache::save_state_final(&format!("task_{}", task_name), state)
}

fn write_log(task: &crate::parser::Task, content: &str) {
    let path = format!("bee/logs/{}.log", task.name);
    let redacted = redact_secrets(content, task);
    if let Err(err) = crate::safety::atomic_write(&path, redacted.as_bytes()) {
        crate::ui::einfo(format!("Warning: could not write log '{}': {}", path, err));
    }
}

fn is_secret_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    [
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "API_KEY",
        "APIKEY",
        "PRIVATE_KEY",
        "CREDENTIAL",
    ]
    .iter()
    .any(|needle| upper.contains(needle))
}

fn redact_secrets(content: &str, task: &crate::parser::Task) -> String {
    let mut secrets: Vec<String> = std::env::vars()
        .filter(|(name, value)| is_secret_name(name) && value.len() >= 4)
        .map(|(_, value)| value)
        .collect();
    if let Some(env) = &task.env {
        secrets.extend(
            env.iter()
                .filter(|(name, value)| is_secret_name(name) && value.len() >= 4)
                .map(|(_, value)| value.clone()),
        );
    }
    secrets.sort_by_key(|value| std::cmp::Reverse(value.len()));
    secrets.dedup();

    let mut redacted = content.to_string();
    for secret in secrets {
        redacted = redacted.replace(&secret, "[REDACTED]");
    }
    redacted
}

fn format_success_log(task_name: &str, result: &crate::execution::ExecutionResult) -> String {
    format!(
        "task '{}' exit code: {}\n[stdout]\n{}\n[stderr]\n{}\n",
        task_name, result.exit_code, result.stdout, result.stderr
    )
}

pub fn run_task(task: crate::parser::Task, force: bool) -> Result<()> {
    cache::begin_task(&task.name)?;

    let should_run = match should_run(&task.name, force) {
        Ok(value) => value,
        Err(err) => {
            cache::discard_pending()?;
            return Err(err);
        }
    };
    if !should_run {
        // A skipped task still made valid observations (e.g. a cold-cache
        // baseline for file_change). Commit them; only a failed task must
        // leave trigger state unchanged so it can retry.
        cache::commit_pending()?;
        crate::ui::info(format!(
            "{}: skipped (no triggers fired; use 'bee task run {} --force' to run anyway)",
            task.name, task.name
        ));
        return Ok(());
    }

    crate::ui::info(format!("{}: running", task.name));

    let result = crate::execution::execute_task(&task);
    match result {
        Ok(exec_result) if exec_result.success => {
            cache::commit_pending()?;
            let log = format_success_log(&task.name, &exec_result);
            write_log(&task, &log);
            let stamp = crate::hash::hash_string(&crate::time::get_timestamp_string());
            if let Err(err) = cache::set_last_run(&task.name, &stamp) {
                crate::ui::einfo(format!("Warning: could not record last run: {}", err));
            }
            mark_task_state(&task.name, "completed")?;
            crate::ui::info(format!("{}: completed", task.name));
            Ok(())
        }
        Ok(exec_result) => {
            cache::discard_pending()?;
            let msg = format_failed_execution(&task.name, &exec_result);
            write_log(&task, &format!("{}\n", msg));
            mark_task_state(&task.name, "failed")?;
            Err(TaskFailure {
                task: task.name,
                exit_code: if exec_result.exit_code < 0 {
                    1
                } else {
                    exec_result.exit_code
                },
                message: msg,
            }
            .into())
        }
        Err(err) => {
            cache::discard_pending()?;
            let msg = err.to_string();
            write_log(&task, &format!("{}\n", msg));
            mark_task_state(&task.name, "failed")?;
            crate::ui::einfo(format!("{}: FAILED ({})", task.name, msg));
            Err(anyhow!("{}", msg))
        }
    }
}

fn format_failed_execution(
    task_name: &str,
    result: &crate::execution::ExecutionResult,
) -> String {
    let mut msg = format!(
        "task '{}' failed with exit code {}",
        task_name, result.exit_code
    );
    if !result.stdout.is_empty() {
        msg.push_str(&format!("\n[stdout]\n{}", result.stdout));
    }
    if !result.stderr.is_empty() {
        msg.push_str(&format!("\n[stderr]\n{}", result.stderr));
    }
    msg
}
