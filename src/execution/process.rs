use crate::execution::context::ExecutionContext;
use crate::execution::error::ExecutionError;
use crate::execution::executor::{ExecutionResult, Executor};
use crate::execution::resolution::resolve_executable;
use std::io::{self, Read, Write};
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
    #[cfg(windows)]
    {
        let is_cmd_c = is_cmd_program(&launch_program)
            && resolved_args.first().map(|s| s.as_str()) == Some("/C");
        let is_cmd_sc = is_cmd_program(&launch_program)
            && resolved_args.first().map(|s| s.as_str()) == Some("/S")
            && resolved_args.get(1).map(|s| s.as_str()) == Some("/C");
        if is_cmd_c || is_cmd_sc {
            use std::os::windows::process::CommandExt;
            if is_cmd_sc {
                cmd.arg("/S").arg("/C");
                if let Some(command_line) = resolved_args.get(2) {
                    cmd.raw_arg(command_line);
                }
                for extra in resolved_args.iter().skip(3) {
                    cmd.arg(extra);
                }
            } else {
                cmd.arg("/C");
                if let Some(command_line) = resolved_args.get(1) {
                    cmd.raw_arg(command_line);
                }
                for extra in resolved_args.iter().skip(2) {
                    cmd.arg(extra);
                }
            }
        } else {
            cmd.args(&resolved_args);
        }
    }
    #[cfg(not(windows))]
    {
        cmd.args(&resolved_args);
    }

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
    let interactive = context.options.interactive_tty;
    let mut child = cmd
        .stdin(Stdio::inherit())
        .stdout(if interactive {
            Stdio::inherit()
        } else {
            Stdio::piped()
        })
        .stderr(if interactive {
            Stdio::inherit()
        } else {
            Stdio::piped()
        })
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

    if interactive {
        let status = wait_for_exit(&mut child, &context.task_name, context.options.timeout)?;
        let duration = start.elapsed();
        return Ok(ExecutionResult {
            success: status.success(),
            exit_code: status.code().unwrap_or(-1),
            stdout: String::new(),
            stderr: String::new(),
            duration,
        });
    }

    // Drain both pipes concurrently while streaming them to the terminal.
    // Streaming keeps output visible in real time and still captures
    // full output for logs. Pipe draining prevents deadlocks on full buffers.
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let task_out = context.task_name.clone();
    let task_err = context.task_name.clone();
    let stdout_thread = thread::spawn(move || tee_reader(stdout_pipe, &task_out));
    let stderr_thread = thread::spawn(move || tee_reader(stderr_pipe, &task_err));
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
            // /S /C with an outer quoted command line gives predictable quote
            // handling for paths/args that contain spaces.
            return (
                "cmd.exe".to_string(),
                vec![
                    "/S".to_string(),
                    "/C".to_string(),
                    format!("\"{}\"", command_line),
                ],
            );
        }
    }

    #[cfg(not(windows))]
    {
        let _ = Path::new(program);
    }

    (program.to_string(), args.to_vec())
}

fn tee_reader(pipe: Option<impl Read>, task: &str) -> Vec<u8> {
    let Some(mut pipe) = pipe else {
        return Vec::new();
    };
    let mut captured = Vec::new();
    let mut buf = [0u8; 4096];
    let mut at_line_start = true;
    let json_mode = crate::ui::json_mode();
    let prefix = if crate::ui::color_enabled() {
        format!("\x1b[36m[{}]\x1b[0m ", task)
    } else {
        format!("[{}] ", task)
    };

    loop {
        match pipe.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let chunk = &buf[..n];
                captured.extend_from_slice(chunk);
                if json_mode {
                    continue;
                }
                let mut out = io::stdout().lock();
                let mut i = 0;
                while i < chunk.len() {
                    if at_line_start {
                        let _ = out.write_all(prefix.as_bytes());
                        at_line_start = false;
                    }
                    let next_nl = chunk[i..].iter().position(|&b| b == b'\n');
                    if let Some(rel) = next_nl {
                        let end = i + rel + 1;
                        let _ = out.write_all(&chunk[i..end]);
                        at_line_start = true;
                        i = end;
                    } else {
                        let _ = out.write_all(&chunk[i..]);
                        i = chunk.len();
                    }
                }
                let _ = out.flush();
            }
        }
    }
    captured
}

/// `resolve_executable` can turn `cmd.exe` into an absolute path, so match on
/// the file name to make sure cmd's argument line is passed through `raw_arg`
/// instead of being escaped by Rust's Windows command-line quoting.
#[cfg(any(windows, test))]
fn is_cmd_program(program: &str) -> bool {
    Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("cmd.exe"))
}

/// Quote a single argument for `cmd.exe /C`. CMD treats `""` inside a quoted
/// string as a literal quote and `"` as a delimiter; this keeps arguments
/// containing spaces or shell metacharacters as one token with no injection.
#[cfg(windows)]
fn quote_cmd_argument(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod program_detection_tests {
    use super::is_cmd_program;

    #[test]
    fn detects_cmd_program_after_path_resolution() {
        assert!(is_cmd_program("cmd.exe"));
        assert!(is_cmd_program("CMD.EXE"));
        assert!(is_cmd_program("C:/Windows/System32/cmd.exe"));
        assert!(!is_cmd_program("C:/Windows/System32/cmd"));
        assert!(!is_cmd_program("C:/Windows/System32/powershell.exe"));
    }
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
