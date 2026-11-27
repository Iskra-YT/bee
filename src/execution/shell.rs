use crate::execution::context::ExecutionContext;
use crate::execution::error::ExecutionError;
use crate::execution::executor::{ExecutionResult, Executor};
use crate::execution::process::run_process;

/// Re-exported for compatibility: executable discovery lives in
/// [`crate::execution::resolution`] and is shared by all executors.
pub use crate::execution::resolution::program_exists;

pub struct ShellExecutor;

impl Executor for ShellExecutor {
    fn execute(&self, context: &ExecutionContext) -> Result<ExecutionResult, ExecutionError> {
        let shell = context
            .shell
            .as_deref()
            .ok_or_else(|| ExecutionError::MissingField {
                task: context.task_name.clone(),
                field: "shell".to_string(),
            })?;
        let command =
            context
                .shell_command
                .as_deref()
                .ok_or_else(|| ExecutionError::MissingField {
                    task: context.task_name.clone(),
                    field: "run".to_string(),
                })?;

        let (program, args) = shell_invocation(shell, command);
        if !program_exists(&program) {
            return Err(ExecutionError::ShellNotFound {
                task: context.task_name.clone(),
                shell: shell.to_string(),
            });
        }

        run_process(context, &program, &args)
    }
}

pub fn shell_invocation(shell: &str, command: &str) -> (String, Vec<String>) {
    let shell_name = std::path::Path::new(shell)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(shell)
        .to_ascii_lowercase();

    match shell_name.as_str() {
        "cmd" | "cmd.exe" => (
            shell.to_string(),
            vec!["/C".to_string(), command.to_string()],
        ),
        "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe" => (
            shell.to_string(),
            vec![
                "-NoProfile".to_string(),
                "-NonInteractive".to_string(),
                "-Command".to_string(),
                command.to_string(),
            ],
        ),
        _ => (
            shell.to_string(),
            vec!["-c".to_string(), command.to_string()],
        ),
    }
}

pub fn default_shell() -> String {
    if cfg!(windows) {
        "cmd".to_string()
    } else {
        "sh".to_string()
    }
}
