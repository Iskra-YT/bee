pub mod reader;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(default)]
pub struct PlatformOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub program: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpreter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct Task {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub program: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpreter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platforms: Option<BTreeMap<String, PlatformOverride>>,
    #[serde(skip_serializing_if = "is_false")]
    pub legacy: bool,
}

impl Task {
    pub fn validate(&self) -> Result<()> {
        let executor = self.effective_executor();

        if self.executor.is_none()
            && self.run.is_some()
            && (self.program.is_some() || self.script.is_some() || self.interpreter.is_some())
        {
            return Err(anyhow!(
                "Task '{}': conflicting configuration: 'run' cannot be combined with 'program/script/interpreter' without explicit 'executor'",
                self.name
            ));
        }

        match executor.as_str() {
            "process" => {
                if self.effective_program().is_none() {
                    return Err(anyhow!(
                        "Task '{}': executor 'process' requires 'program'",
                        self.name
                    ));
                }
                if self.run.is_some() {
                    return Err(anyhow!(
                        "Task '{}': executor 'process' cannot be combined with 'run'",
                        self.name
                    ));
                }
                if self.shell.is_some() || self.interpreter.is_some() || self.script.is_some() {
                    return Err(anyhow!(
                        "Task '{}': executor 'process' does not accept 'shell/interpreter/script'",
                        self.name
                    ));
                }
            }
            "shell" => {
                if self.effective_run().is_none() {
                    return Err(anyhow!(
                        "Task '{}': executor 'shell' requires 'run'",
                        self.name
                    ));
                }
                if self.program.is_some()
                    || self.script.is_some()
                    || self.interpreter.is_some()
                    || self.args.is_some()
                {
                    return Err(anyhow!("Task '{}': executor 'shell' only supports 'run', 'shell', 'working_dir', 'env', and 'platforms'", self.name));
                }
            }
            "script" => {
                if self.effective_interpreter().is_none() {
                    return Err(anyhow!(
                        "Task '{}': executor 'script' requires 'interpreter'",
                        self.name
                    ));
                }
                if self.effective_script().is_none() {
                    return Err(anyhow!(
                        "Task '{}': executor 'script' requires 'script'",
                        self.name
                    ));
                }
                if self.run.is_some() || self.program.is_some() || self.shell.is_some() {
                    return Err(anyhow!(
                        "Task '{}': executor 'script' cannot be combined with 'run/program/shell'",
                        self.name
                    ));
                }
            }
            "legacy" => {
                if self.run.is_none() {
                    return Err(anyhow!("Task '{}': legacy task requires 'run'", self.name));
                }
            }
            other => {
                return Err(anyhow!(
                    "Task '{}': unknown executor '{}'",
                    self.name,
                    other
                ));
            }
        }

        if let Some(platforms) = &self.platforms {
            for (key, override_cfg) in platforms {
                match key.as_str() {
                    "linux" | "windows" | "macos" => {}
                    _ => {
                        return Err(anyhow!(
                            "Task '{}': invalid platform '{}' in platforms; supported: linux, windows, macos",
                            self.name, key
                        ));
                    }
                }
                if let Some(exec) = &override_cfg.executor {
                    match exec.as_str() {
                        "process" | "shell" | "script" => {}
                        _ => {
                            return Err(anyhow!(
                                "Task '{}': invalid executor '{}' in platforms.{}",
                                self.name,
                                exec,
                                key
                            ))
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn effective_executor(&self) -> String {
        if let Some(exec) = &self.executor {
            return exec.clone();
        }
        if self.program.is_some() {
            return "process".to_string();
        }
        if self.script.is_some() || self.interpreter.is_some() {
            return "script".to_string();
        }
        if self.run.is_some() {
            return "legacy".to_string();
        }
        "legacy".to_string()
    }

    pub fn effective_program(&self) -> Option<String> {
        self.program.clone()
    }

    pub fn effective_run(&self) -> Option<String> {
        self.run.clone()
    }

    pub fn effective_interpreter(&self) -> Option<String> {
        self.interpreter.clone()
    }

    pub fn effective_script(&self) -> Option<String> {
        self.script.clone()
    }

    pub fn resolved_for_current_platform(&self) -> Result<Task> {
        let os = std::env::consts::OS;
        let platform_key = match os {
            "linux" | "windows" | "macos" => os,
            _ => os,
        };

        let mut resolved = self.clone();
        if let Some(platforms) = &self.platforms {
            // If platforms are specified and current OS is not listed, we still allow fallback to default
            // unless the task explicitly wants platform-only config; but we keep it simple: fallback to default.
            if let Some(override_cfg) = platforms.get(platform_key) {
                if let Some(v) = &override_cfg.executor {
                    resolved.executor = Some(v.clone());
                }
                if let Some(v) = &override_cfg.run {
                    resolved.run = Some(v.clone());
                }
                if let Some(v) = &override_cfg.program {
                    resolved.program = Some(v.clone());
                }
                if let Some(v) = &override_cfg.args {
                    resolved.args = Some(v.clone());
                }
                if let Some(v) = &override_cfg.shell {
                    resolved.shell = Some(v.clone());
                }
                if let Some(v) = &override_cfg.interpreter {
                    resolved.interpreter = Some(v.clone());
                }
                if let Some(v) = &override_cfg.script {
                    resolved.script = Some(v.clone());
                }
                if let Some(v) = &override_cfg.working_dir {
                    resolved.working_dir = Some(v.clone());
                }
                if let Some(v) = &override_cfg.env {
                    let mut env = resolved.env.unwrap_or_default();
                    for (k, val) in v {
                        env.insert(k.clone(), val.clone());
                    }
                    resolved.env = Some(env);
                }
            }
        }

        resolved.validate()?;
        Ok(resolved)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pipeline {
    pub name: String,
    pub tasks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pipelines: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rule {
    pub name: String,
    pub task: String,
    #[serde(default)]
    pub triggers: Vec<RuleTrigger>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum RuleTrigger {
    #[serde(rename = "file_change")]
    FileChange { paths: Vec<String> },
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "schedule")]
    Schedule { cron: String },
    #[serde(rename = "task_completed")]
    TaskCompleted { task: String },
    #[serde(rename = "task_failed")]
    TaskFailed { task: String },
    #[serde(rename = "dependency_changed")]
    DependencyChanged { task: String },
    #[serde(rename = "git_changed")]
    GitChanged { paths: Vec<String> },
    #[serde(rename = "git_tag")]
    GitTag { pattern: Option<String> },
    #[serde(rename = "env_changed")]
    EnvChanged { vars: Vec<String> },
    #[serde(rename = "dependency_missing")]
    DependencyMissing { path: String },
    #[serde(rename = "checksum_changed")]
    ChecksumChanged { paths: Vec<String> },
    #[serde(rename = "subtask_completed")]
    SubtaskCompleted { subsystem: String, task: String },
    #[serde(rename = "subtask_failed")]
    SubtaskFailed { subsystem: String, task: String },
    #[serde(rename = "subtask_changed")]
    SubtaskChanged { subsystem: String, task: String },
    #[serde(rename = "parent_task_completed")]
    ParentTaskCompleted { task: String },
    #[serde(rename = "parent_task_failed")]
    ParentTaskFailed { task: String },
    #[serde(rename = "subsystem_changed")]
    SubsystemChanged { subsystem: String },
}

#[derive(Serialize)]
pub struct MainConfig {
    pub tasks: Vec<String>,
    pub rules: Vec<String>,
    pub pipelines: Vec<String>,
}

pub struct PipelineConfig {
    pub name: String,
}

pub struct TaskConfig {
    pub name: String,
}

pub struct RuleConfig {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dependency {
    pub name: String,
    #[serde(rename = "type")]
    pub dep_type: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub install_script: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subsystem {
    pub name: String,
    pub path: String,
    pub date: String,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SubsystemsConfig {
    #[serde(default)]
    pub subsystems: Vec<Subsystem>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DepsConfig {
    pub dependencies: Vec<Dependency>,
}
