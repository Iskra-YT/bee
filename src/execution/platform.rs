use crate::parser::{PlatformOverride, Task};
use anyhow::{anyhow, Result};
use std::collections::BTreeMap;

pub fn current_platform() -> &'static str {
    std::env::consts::OS
}

pub fn resolve_task_for_platform(task: &Task) -> Result<Task> {
    let platform = current_platform();
    let mut resolved = task.clone();

    if let Some(platforms) = &task.platforms {
        validate_platform_keys(task, platforms)?;
        if let Some(override_cfg) = platforms.get(platform) {
            apply_override(&mut resolved, override_cfg);
        }
    }

    resolved.validate().map_err(|e| anyhow!("{}", e))?;
    Ok(resolved)
}

fn validate_platform_keys(
    task: &Task,
    platforms: &BTreeMap<String, PlatformOverride>,
) -> Result<()> {
    for key in platforms.keys() {
        match key.as_str() {
            "linux" | "windows" | "macos" => {}
            _ => {
                return Err(anyhow!(
                "Task '{}': invalid platform '{}' in platforms; supported: linux, windows, macos",
                task.name,
                key
            ))
            }
        }
    }
    Ok(())
}

fn apply_override(task: &mut Task, override_cfg: &PlatformOverride) {
    if let Some(v) = &override_cfg.executor {
        task.executor = Some(v.clone());
    }
    if let Some(v) = &override_cfg.run {
        task.run = Some(v.clone());
    }
    if let Some(v) = &override_cfg.program {
        task.program = Some(v.clone());
    }
    if let Some(v) = &override_cfg.args {
        task.args = Some(v.clone());
    }
    if let Some(v) = &override_cfg.shell {
        task.shell = Some(v.clone());
    }
    if let Some(v) = &override_cfg.interpreter {
        task.interpreter = Some(v.clone());
    }
    if let Some(v) = &override_cfg.script {
        task.script = Some(v.clone());
    }
    if let Some(v) = &override_cfg.working_dir {
        task.working_dir = Some(v.clone());
    }
    if let Some(v) = &override_cfg.env {
        let mut env = task.env.clone().unwrap_or_default();
        for (k, val) in v {
            env.insert(k.clone(), val.clone());
        }
        task.env = Some(env);
    }
}
