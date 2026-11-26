use crate::execution::context::ExecutionContext;
use crate::execution::error::ExecutionError;
use crate::execution::executor::{ExecutionResult, Executor};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

pub struct ProcessExecutor;

impl Executor for ProcessExecutor {
    fn execute(&self, context: &ExecutionContext) -> Result<ExecutionResult, ExecutionError> {
        run_process(context, &context.program, &context.args)
    }
}

pub fn run_process(
    context: &ExecutionContext,
    program: &str,
    args: &[String],
) -> Result<ExecutionResult, ExecutionError> {
    let (resolved_program, resolved_args) = resolve_program_invocation(program, args);
    let mut cmd = Command::new(&resolved_program);
    cmd.args(&resolved_args);

    if let Some(dir) = &context.working_dir {
        if !dir.exists() {
            return Err(ExecutionError::WorkingDirectory {
                task: context.task_name.clone(),
                path: dir.display().to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "working directory does not exist",
                ),
            });
        }
        cmd.current_dir(dir);
    }

    if !context.inherit_env {
        cmd.env_clear();
    }

    for (k, v) in &context.env {
        cmd.env(k, v);
    }

    let start = Instant::now();
    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ExecutionError::ProgramNotFound {
                task: context.task_name.clone(),
                program: program.to_string(),
            }
        } else {
            ExecutionError::Spawn {
                task: context.task_name.clone(),
                program: program.to_string(),
                source: e,
            }
        }
    })?;
    let duration = start.elapsed();

    let exit_code = output.status.code().unwrap_or(-1);
    Ok(ExecutionResult {
        success: output.status.success(),
        exit_code,
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        duration,
    })
}

fn resolve_program_invocation(program: &str, args: &[String]) -> (String, Vec<String>) {
    #[cfg(windows)]
    {
        let lower = Path::new(program)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if lower == "cmd" || lower == "bat" {
            let mut cmd_args = Vec::with_capacity(args.len() + 2);
            cmd_args.push("/C".to_string());
            cmd_args.push(program.to_string());
            cmd_args.extend(args.iter().cloned());
            return ("cmd.exe".to_string(), cmd_args);
        }
    }

    #[cfg(not(windows))]
    {
        let _ = Path::new(program);
    }

    (program.to_string(), args.to_vec())
}
