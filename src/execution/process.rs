use crate::execution::context::ExecutionContext;
use crate::execution::error::ExecutionError;
use crate::execution::executor::{ExecutionResult, Executor};
use crate::execution::resolution::resolve_executable;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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
    let launch_program = resolve_executable(&resolved_program);
    let mut cmd = Command::new(&launch_program);
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
        // A cleared environment must still contain the variables needed to
        // launch the process and find its temporary directory.
        for key in essential_env_keys() {
            if let Some(value) = std::env::var_os(key) {
                cmd.env(key, value);
            }
        }
    }

    for (k, v) in &context.env {
        cmd.env(k, v);
    }

    let start = Instant::now();
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
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

    // Drain both pipes concurrently. Otherwise a child that writes more than
    // a pipe buffer to stderr while we wait for exit would deadlock.
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();
    let stdout_thread = thread::spawn(move || {
        let mut output = Vec::new();
        if let Some(pipe) = stdout_pipe.as_mut() {
            let _ = pipe.read_to_end(&mut output);
        }
        output
    });
    let stderr_thread = thread::spawn(move || {
        let mut output = Vec::new();
        if let Some(pipe) = stderr_pipe.as_mut() {
            let _ = pipe.read_to_end(&mut output);
        }
        output
    });

    let wait_result = wait_for_exit(&mut child, &context.task_name, context.options.timeout);
    let stdout = stdout_thread.join().unwrap_or_default();
    let stderr = stderr_thread.join().unwrap_or_default();
    let status = wait_result?;
    let duration = start.elapsed();

    let exit_code = status.code().unwrap_or(-1);
    Ok(ExecutionResult {
        success: status.success(),
        exit_code,
        stdout: String::from_utf8_lossy(&stdout).to_string(),
        stderr: String::from_utf8_lossy(&stderr).to_string(),
        duration,
    })
}

fn wait_for_exit(
    child: &mut Child,
    task: &str,
    timeout: Option<Duration>,
) -> Result<ExitStatus, ExecutionError> {
    let wait_error = |source: std::io::Error| ExecutionError::Io {
        task: task.to_string(),
        message: "could not wait for the task process".to_string(),
        source,
    };
    let Some(limit) = timeout else {
        return child.wait().map_err(wait_error);
    };

    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(wait_error)? {
            return Ok(status);
        }
        if start.elapsed() >= limit {
            // Kill the direct child; readers observe EOF once it exits.
            let _ = child.kill();
            let _ = child.wait();
            return Err(ExecutionError::Timeout { duration: limit });
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(windows)]
fn essential_env_keys() -> &'static [&'static str] {
    &[
        "PATH",
        "PATHEXT",
        "SystemRoot",
        "SystemDrive",
        "TEMP",
        "TMP",
        "USERPROFILE",
        "HOMEDRIVE",
        "HOMEPATH",
        "COMSPEC",
        "NUMBER_OF_PROCESSORS",
    ]
}

#[cfg(not(windows))]
fn essential_env_keys() -> &'static [&'static str] {
    &["PATH", "HOME", "TMPDIR", "LANG", "LC_ALL"]
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
            let command_line = std::iter::once(program.to_string())
                .chain(args.iter().cloned())
                .map(|part| quote_cmd_argument(&part))
                .collect::<Vec<_>>()
                .join(" ");
            return ("cmd.exe".to_string(), vec!["/C".to_string(), command_line]);
        }
    }

    #[cfg(not(windows))]
    {
        let _ = Path::new(program);
    }

    (program.to_string(), args.to_vec())
}

/// Quote a single argument for `cmd.exe /C`. CMD treats `""` inside a quoted
/// string as a literal quote and `"` as a delimiter; this keeps arguments
/// containing spaces or shell metacharacters as one token with no injection.
#[cfg(windows)]
fn quote_cmd_argument(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn quotes_cmd_arguments() {
        assert_eq!(quote_cmd_argument("a b"), "\"a b\"");
        assert_eq!(quote_cmd_argument("a\"b"), "\"a\"\"b\"");
    }
}
