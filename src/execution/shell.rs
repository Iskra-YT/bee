use crate::execution::context::ExecutionContext;
use crate::execution::error::ExecutionError;
use crate::execution::executor::{ExecutionResult, Executor};
use crate::execution::process::run_process;

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

pub fn program_exists(program: &str) -> bool {
    if std::path::Path::new(program).components().count() > 1 {
        return std::path::Path::new(program).exists();
    }

    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };

    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return true;
        }
        #[cfg(windows)]
        {
            let pathext =
                std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".to_string());
            for ext in pathext.split(';').filter(|e| !e.is_empty()) {
                let ext = if ext.starts_with('.') {
                    ext.to_string()
                } else {
                    format!(".{}", ext)
                };
                let candidate = dir.join(format!("{}{}", program, ext));
                if candidate.is_file() {
                    return true;
                }
            }
        }
    }
    false
}
