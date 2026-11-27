use crate::execution::context::ExecutionContext;
use crate::execution::error::ExecutionError;
use crate::execution::executor::{ExecutionResult, Executor};
use crate::execution::platform::resolve_task_for_platform;
use crate::execution::process::ProcessExecutor;
use crate::execution::script::ScriptExecutor;
use crate::execution::shell::{default_shell, ShellExecutor};
use crate::parser::Task;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub struct ExecutorRegistry;

impl Default for ExecutorRegistry {
    fn default() -> Self {
        Self
    }
}

impl ExecutorRegistry {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        task: &Task,
        interactive: bool,
    ) -> Result<ExecutionResult, ExecutionError> {
        let resolved =
            resolve_task_for_platform(task).map_err(|e| ExecutionError::InvalidConfig {
                task: task.name.clone(),
                message: e.to_string(),
            })?;

        let executor_name = resolved.effective_executor();
        let mut context = build_context(&resolved)?;
        context.options.interactive_tty = interactive;

        match executor_name.as_str() {
            "process" => ProcessExecutor.execute(&context),
            "shell" => {
                context.shell = resolved.shell.clone().or_else(|| Some(default_shell()));
                context.shell_command = resolved.run.clone();
                ShellExecutor.execute(&context)
            }
            "legacy" => {
                // Use the platform shell: `sh -c` on Unix, `cmd.exe /C` on
                // Windows where `sh` is usually unavailable.
                context.shell = Some(default_shell());
                context.shell_command = resolved.run.clone();
                ShellExecutor.execute(&context)
            }
            "script" => ScriptExecutor.execute(&context),
            other => Err(ExecutionError::UnsupportedExecutor {
                task: task.name.clone(),
                executor: other.to_string(),
            }),
        }
    }
}

fn build_context(task: &Task) -> Result<ExecutionContext, ExecutionError> {
    let program = task.program.clone().unwrap_or_default();
    let mut context = ExecutionContext::new(task.name.clone(), program);
    context.args = task.args.clone().unwrap_or_default();
    context.working_dir = task.working_dir.as_ref().map(|dir| {
        let path = PathBuf::from(dir);
        if path.is_absolute() {
            path
        } else {
            // Relative directories are always anchored to the project root —
            // the directory that contains `bee/` — not to a task's script path.
            let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            root.join(path)
        }
    });
    context.inherit_env = task.inherit_env.unwrap_or(true);
    context.options.timeout =
        match &task.timeout {
            Some(value) => Some(crate::time::parse_duration(value).map_err(|e| {
                ExecutionError::InvalidConfig {
                    task: task.name.clone(),
                    message: e.to_string(),
                }
            })?),
            None => None,
        };
    context.env = build_env(task);
    context.interpreter = task.interpreter.clone();
    context.interpreter_candidates = task.interpreter_candidates();
    context.script = task.script.clone();
    Ok(context)
}

fn build_env(task: &Task) -> BTreeMap<String, String> {
    let mut env = task.env.clone().unwrap_or_default();
    if let Some(owner) = crate::subsystem::deps_owner_dir() {
        env.entry("BEE_DEPS_PATH".to_string())
            .or_insert_with(|| owner.join("bee/deps").to_string_lossy().to_string());
    }
    env
}

pub fn execute_task(task: &Task, interactive: bool) -> Result<ExecutionResult, ExecutionError> {
    ExecutorRegistry::new().execute(task, interactive)
}
