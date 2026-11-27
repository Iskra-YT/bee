use crate::hash;
use crate::safety;
use anyhow::Result;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

const TRIGGERS_DIR: &str = "bee/cache/triggers";

thread_local! {
    /// Trigger state written while evaluating a task is staged here. It is
    /// committed only after the task succeeds, so a failed task can retry.
    static CURRENT_TASK: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn trigger_dir(base: &str) -> PathBuf {
    Path::new(base).join(TRIGGERS_DIR)
}

fn state_path(base: &str, key: &str) -> PathBuf {
    trigger_dir(base).join(hash::hash_string(key))
}

fn staging_dir(task: &str) -> PathBuf {
    trigger_dir(".").join(".pending").join(hash::hash_string(task))
}

pub fn get_state(key: &str) -> Option<String> {
    fs::read_to_string(state_path(".", key)).ok()
}

pub fn get_state_at(base_dir: &str, key: &str) -> Option<String> {
    fs::read_to_string(state_path(base_dir, key)).ok()
}

/// Save a trigger state. During task evaluation the write is staged and must
/// be committed with `commit_pending()` after the task succeeds.
pub fn save_state(key: &str, value: &str) -> Result<()> {
    let staged = CURRENT_TASK.with(|current| current.borrow().clone());
    let path = match staged {
        Some(task) => staging_dir(&task).join(hash::hash_string(key)),
        None => state_path(".", key),
    };
    safety::atomic_write(path, value.as_bytes())
}

/// Final write that is never staged (used for task result state).
pub fn save_state_final(key: &str, value: &str) -> Result<()> {
    safety::atomic_write(state_path(".", key), value.as_bytes())
}

/// Start a task: discard any staged state left over from an interrupted run.
pub fn begin_task(name: &str) -> Result<()> {
    let dir = staging_dir(name);
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    CURRENT_TASK.with(|current| *current.borrow_mut() = Some(name.to_string()));
    Ok(())
}

/// Promote staged trigger states to real states after a successful task.
pub fn commit_pending() -> Result<()> {
    let task = CURRENT_TASK.with(|current| current.borrow().clone());
    if let Some(task) = task {
        let dir = staging_dir(&task);
        if dir.exists() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                if !entry.file_type()?.is_file() {
                    continue;
                }
                let value = fs::read_to_string(entry.path())?;
                let key = entry.file_name().to_string_lossy().to_string();
                safety::atomic_write(trigger_dir(".").join(key), value.as_bytes())?;
            }
            fs::remove_dir_all(&dir)?;
        }
    }
    CURRENT_TASK.with(|current| *current.borrow_mut() = None);
    Ok(())
}

/// Drop staged trigger states after a failed or skipped task.
pub fn discard_pending() -> Result<()> {
    let task = CURRENT_TASK.with(|current| current.borrow().clone());
    if let Some(task) = task {
        let dir = staging_dir(&task);
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
    }
    CURRENT_TASK.with(|current| *current.borrow_mut() = None);
    Ok(())
}

/// Path to the cached `last_run` marker for a task.
pub fn last_run_path(name: &str) -> PathBuf {
    Path::new("bee/cache")
        .join(hash::hash_string(name))
        .join("last_run")
}

/// Record when a task last ran for `bee status`.
pub fn set_last_run(name: &str, value: &str) -> Result<()> {
    safety::atomic_write(last_run_path(name), value.as_bytes())
}
