pub mod reader;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn is_false(value: &bool) -> bool {
    !*value
}

/// The value of the `interpreter` configuration key.
///
/// A single string names one interpreter; a list contains alternatives. Bee
/// selects the first interpreter available on the current platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InterpreterSpec {
    Single(String),
    List(Vec<String>),
}

impl InterpreterSpec {
    pub fn as_slice(&self) -> &[String] {
        match self {
            InterpreterSpec::Single(value) => std::slice::from_ref(value),
            InterpreterSpec::List(values) => values,
        }
    }

    pub fn into_vec(self) -> Vec<String> {
        match self {
            InterpreterSpec::Single(value) => vec![value],
            InterpreterSpec::List(values) => values,
        }
    }
}

fn split_interpreter_spec(spec: Option<InterpreterSpec>) -> (Option<String>, Option<Vec<String>>) {
    match spec {
        Some(InterpreterSpec::Single(value)) => (Some(value), None),
        Some(InterpreterSpec::List(values)) => (None, Some(values)),
        None => (None, None),
    }
}

fn join_interpreter_spec(
    interpreter: Option<&String>,
    interpreter_candidates: Option<&Vec<String>>,
) -> Option<InterpreterSpec> {
    if let Some(candidates) = interpreter_candidates {
        Some(InterpreterSpec::List(candidates.clone()))
    } else {
        interpreter.cloned().map(InterpreterSpec::Single)
    }
}

fn validate_interpreter_candidates(task: &str, candidates: &[String]) -> Result<()> {
    if candidates.is_empty() {
        return Err(anyhow!(
            "Task '{}': 'interpreter' list cannot be empty; specify at least one interpreter",
            task
        ));
    }
    if candidates
        .iter()
        .any(|candidate| candidate.trim().is_empty())
    {
        return Err(anyhow!(
            "Task '{}': 'interpreter' list cannot contain empty values",
            task
        ));
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct PlatformOverrideFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    executor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    run: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    program: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shell: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    interpreter: Option<InterpreterSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    script: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    working_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(from = "PlatformOverrideFields", into = "PlatformOverrideFields")]
pub struct PlatformOverride {
    pub executor: Option<String>,
    pub run: Option<String>,
    pub program: Option<String>,
    pub args: Option<Vec<String>>,
    pub shell: Option<String>,
    pub interpreter: Option<String>,
    /// Alternative interpreters, when configured as a YAML list.
    pub interpreter_candidates: Option<Vec<String>>,
    pub script: Option<String>,
    pub working_dir: Option<String>,
    pub env: Option<BTreeMap<String, String>>,
}

impl From<PlatformOverrideFields> for PlatformOverride {
    fn from(fields: PlatformOverrideFields) -> Self {
        let (interpreter, interpreter_candidates) = split_interpreter_spec(fields.interpreter);
        Self {
            executor: fields.executor,
            run: fields.run,
            program: fields.program,
            args: fields.args,
            shell: fields.shell,
            interpreter,
            interpreter_candidates,
            script: fields.script,
            working_dir: fields.working_dir,
            env: fields.env,
        }
    }
}

impl From<PlatformOverride> for PlatformOverrideFields {
    fn from(value: PlatformOverride) -> Self {
        Self {
            executor: value.executor,
            run: value.run,
            program: value.program,
            args: value.args,
            shell: value.shell,
            interpreter: join_interpreter_spec(
                value.interpreter.as_ref(),
                value.interpreter_candidates.as_ref(),
            ),
            script: value.script,
            working_dir: value.working_dir,
            env: value.env,
        }
    }
}

impl PlatformOverride {
    pub fn interpreter_spec(&self) -> Option<InterpreterSpec> {
        join_interpreter_spec(
            self.interpreter.as_ref(),
            self.interpreter_candidates.as_ref(),
        )
    }

    pub fn set_interpreter_spec(&mut self, spec: InterpreterSpec) {
        let (interpreter, interpreter_candidates) = split_interpreter_spec(Some(spec));
        self.interpreter = interpreter;
        self.interpreter_candidates = interpreter_candidates;
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct TaskFields {
    name: String,
    run: Option<String>,
    depends_on: Option<Vec<String>>,
    executor: Option<String>,
    program: Option<String>,
    args: Option<Vec<String>>,
    shell: Option<String>,
    interpreter: Option<InterpreterSpec>,
    script: Option<String>,
    working_dir: Option<String>,
    env: Option<BTreeMap<String, String>>,
    platforms: Option<BTreeMap<String, PlatformOverride>>,
    #[serde(skip_serializing_if = "is_false")]
    legacy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(from = "TaskFields", into = "TaskFields")]
pub struct Task {
    pub name: String,
    pub run: Option<String>,
    pub depends_on: Option<Vec<String>>,
    pub executor: Option<String>,
    pub program: Option<String>,
    pub args: Option<Vec<String>>,
    pub shell: Option<String>,
    pub interpreter: Option<String>,
    /// Alternative interpreters, when configured as a YAML list.
    pub interpreter_candidates: Option<Vec<String>>,
    pub script: Option<String>,
    pub working_dir: Option<String>,
    pub env: Option<BTreeMap<String, String>>,
    pub platforms: Option<BTreeMap<String, PlatformOverride>>,
    pub legacy: bool,
}

impl From<TaskFields> for Task {
    fn from(fields: TaskFields) -> Self {
        let (interpreter, interpreter_candidates) = split_interpreter_spec(fields.interpreter);
        Self {
            name: fields.name,
            run: fields.run,
            depends_on: fields.depends_on,
            executor: fields.executor,
            program: fields.program,
            args: fields.args,
            shell: fields.shell,
            interpreter,
            interpreter_candidates,
            script: fields.script,
            working_dir: fields.working_dir,
            env: fields.env,
            platforms: fields.platforms,
            legacy: fields.legacy,
        }
    }
}

impl From<Task> for TaskFields {
    fn from(value: Task) -> Self {
        Self {
            name: value.name,
            run: value.run,
            depends_on: value.depends_on,
            executor: value.executor,
            program: value.program,
            args: value.args,
            shell: value.shell,
            interpreter: join_interpreter_spec(
                value.interpreter.as_ref(),
                value.interpreter_candidates.as_ref(),
            ),
            script: value.script,
            working_dir: value.working_dir,
            env: value.env,
            platforms: value.platforms,
            legacy: value.legacy,
        }
    }
}

impl Task {
    pub fn validate(&self) -> Result<()> {
        let executor = self.effective_executor();

        if self.executor.is_none()
            && self.run.is_some()
            && (self.program.is_some()
                || self.script.is_some()
                || self.interpreter.is_some()
                || self.interpreter_candidates.is_some())
        {
            return Err(anyhow!(
                "Task '{}': conflicting configuration: 'run' cannot be combined with 'program/script/interpreter' without explicit 'executor'",
                self.name
            ));
        }

        if let Some(candidates) = &self.interpreter_candidates {
            validate_interpreter_candidates(&self.name, candidates)?;
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
                if self.shell.is_some()
                    || self.interpreter.is_some()
                    || self.interpreter_candidates.is_some()
                    || self.script.is_some()
                {
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
                    || self.interpreter_candidates.is_some()
                    || self.args.is_some()
                {
                    return Err(anyhow!("Task '{}': executor 'shell' only supports 'run', 'shell', 'working_dir', 'env', and 'platforms'", self.name));
                }
            }
            "script" => {
                if self.interpreter_spec().is_none() {
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
                if let Some(candidates) = &override_cfg.interpreter_candidates {
                    validate_interpreter_candidates(
                        &format!("{} (platforms.{})", self.name, key),
                        candidates,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Current `interpreter` setting as a string/list union.
    pub fn interpreter_spec(&self) -> Option<InterpreterSpec> {
        join_interpreter_spec(
            self.interpreter.as_ref(),
            self.interpreter_candidates.as_ref(),
        )
    }

    /// Replace the `interpreter` setting with a string or list specification.
    pub fn set_interpreter_spec(&mut self, spec: InterpreterSpec) {
        let (interpreter, interpreter_candidates) = split_interpreter_spec(Some(spec));
        self.interpreter = interpreter;
        self.interpreter_candidates = interpreter_candidates;
    }

    /// Interpreter candidates in priority order. A single string is treated as
    /// one candidate so discovery is handled uniformly for all script tasks.
    pub fn interpreter_candidates(&self) -> Vec<String> {
        if let Some(candidates) = &self.interpreter_candidates {
            return candidates.clone();
        }
        self.interpreter.clone().into_iter().collect()
    }

    pub fn effective_executor(&self) -> String {
        if let Some(exec) = &self.executor {
            return exec.clone();
        }
        if self.program.is_some() {
            return "process".to_string();
        }
        if self.script.is_some() || self.interpreter_spec().is_some() {
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
        self.interpreter.clone().or_else(|| {
            self.interpreter_candidates
                .as_ref()
                .and_then(|c| c.first().cloned())
        })
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
                if let Some(v) = &override_cfg.interpreter_candidates {
                    resolved.set_interpreter_spec(InterpreterSpec::List(v.clone()));
                } else if let Some(v) = &override_cfg.interpreter {
                    resolved.set_interpreter_spec(InterpreterSpec::Single(v.clone()));
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
