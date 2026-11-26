use bee::execution::{execute_task, ExecutionError};
use bee::parser::Task;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "bee-test-{}-{}-{}",
        name,
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn base_task(name: &str) -> Task {
    Task {
        name: name.to_string(),
        ..Task::default()
    }
}

#[test]
fn parses_legacy_task() {
    let yaml = r#"
name: build
run: echo hello
depends_on:
  - dep1
"#;
    let task: Task = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(task.name, "build");
    assert_eq!(task.run.as_deref(), Some("echo hello"));
    assert_eq!(task.depends_on.as_ref().unwrap(), &vec!["dep1".to_string()]);
    assert_eq!(task.effective_executor(), "legacy");
    task.validate().unwrap();
}

#[test]
fn parses_process_task() {
    let yaml = r#"
name: build
executor: process
program: cargo
args:
  - build
  - --release
working_dir: .
"#;
    let task: Task = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(task.effective_executor(), "process");
    assert_eq!(task.program.as_deref(), Some("cargo"));
    assert_eq!(
        task.args.as_ref().unwrap(),
        &vec!["build".to_string(), "--release".to_string()]
    );
    task.validate().unwrap();
}

#[test]
fn parses_shell_task() {
    let yaml = r#"
name: complex
executor: shell
shell: bash
run: echo hi && echo bye
"#;
    let task: Task = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(task.effective_executor(), "shell");
    assert_eq!(task.shell.as_deref(), Some("bash"));
    task.validate().unwrap();
}

#[test]
fn parses_script_task() {
    let yaml = r#"
name: report
executor: script
interpreter: python
script: scripts/report.py
args:
  - --format
  - json
"#;
    let task: Task = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(task.effective_executor(), "script");
    assert_eq!(task.interpreter.as_deref(), Some("python"));
    assert_eq!(task.script.as_deref(), Some("scripts/report.py"));
    task.validate().unwrap();
}

#[test]
fn validates_missing_process_program() {
    let mut task = base_task("t");
    task.executor = Some("process".to_string());
    let err = task.validate().unwrap_err().to_string();
    assert!(err.contains("requires 'program'"));
}

#[test]
fn validates_missing_shell_run() {
    let mut task = base_task("t");
    task.executor = Some("shell".to_string());
    let err = task.validate().unwrap_err().to_string();
    assert!(err.contains("requires 'run'"));
}

#[test]
fn validates_missing_script_fields() {
    let mut task = base_task("t");
    task.executor = Some("script".to_string());
    task.interpreter = Some("python".to_string());
    let err = task.validate().unwrap_err().to_string();
    assert!(err.contains("requires 'script'"));
}

#[test]
fn validates_unknown_executor() {
    let mut task = base_task("t");
    task.executor = Some("weird".to_string());
    task.run = Some("echo hi".to_string());
    let err = task.validate().unwrap_err().to_string();
    assert!(err.contains("unknown executor"));
}

#[test]
fn validates_invalid_platform() {
    let mut task = base_task("t");
    task.executor = Some("process".to_string());
    task.program = Some("echo".to_string());
    let mut platforms = BTreeMap::new();
    platforms.insert(
        "plan9".to_string(),
        bee::parser::PlatformOverride::default(),
    );
    task.platforms = Some(platforms);
    let err = task.validate().unwrap_err().to_string();
    assert!(err.contains("invalid platform"));
}

#[test]
fn platform_override_selects_current_os() {
    let mut task = base_task("t");
    task.executor = Some("process".to_string());
    task.program = Some("default-program".to_string());

    let override_cfg = bee::parser::PlatformOverride {
        program: Some("platform-program".to_string()),
        ..bee::parser::PlatformOverride::default()
    };
    let mut platforms = BTreeMap::new();
    platforms.insert(std::env::consts::OS.to_string(), override_cfg);
    task.platforms = Some(platforms);

    let resolved = bee::execution::platform::resolve_task_for_platform(&task).unwrap();
    assert_eq!(resolved.program.as_deref(), Some("platform-program"));
}

#[test]
fn legacy_execution_uses_run() {
    let mut task = base_task("legacy");
    if cfg!(windows) {
        task.run = Some("echo hello".to_string());
        // Legacy uses sh -c; on windows sh may not exist. Skip semantics test there.
        // We still check parsing/validation above.
        return;
    }
    task.run = Some("printf legacy-ok".to_string());
    let result = execute_task(&task).unwrap();
    assert!(result.success);
    assert!(result.stdout.contains("legacy-ok"));
}

#[test]
fn process_executor_runs_with_args() {
    let mut task = base_task("proc");
    task.executor = Some("process".to_string());
    if cfg!(windows) {
        task.program = Some("cmd".to_string());
        task.args = Some(vec![
            "/C".to_string(),
            "echo".to_string(),
            "hello".to_string(),
        ]);
    } else {
        task.program = Some("sh".to_string());
        task.args = Some(vec!["-c".to_string(), "printf '%s' hello".to_string()]);
    }
    let result = execute_task(&task).unwrap();
    assert!(result.success);
    assert!(result.stdout.to_lowercase().contains("hello"));
}

#[test]
fn process_executor_preserves_argument_boundaries() {
    let mut task = base_task("args");
    task.executor = Some("process".to_string());
    if cfg!(windows) {
        task.program = Some("cmd".to_string());
        task.args = Some(vec![
            "/C".to_string(),
            "echo".to_string(),
            "a b".to_string(),
        ]);
    } else {
        task.program = Some("sh".to_string());
        task.args = Some(vec![
            "-c".to_string(),
            "printf '[%s]' \"$1\"".to_string(),
            "x".to_string(),
            "a b".to_string(),
        ]);
    }
    let result = execute_task(&task).unwrap();
    assert!(result.success);
    assert!(result.stdout.contains("a b"));
}

#[test]
fn process_executor_working_dir_and_env() {
    let dir = temp_dir("wd");
    let mut task = base_task("wd");
    task.executor = Some("process".to_string());
    task.working_dir = Some(dir.to_string_lossy().to_string());
    let mut env = BTreeMap::new();
    env.insert("BEE_TEST_VAR".to_string(), "bee-value".to_string());
    task.env = Some(env);

    if cfg!(windows) {
        task.program = Some("cmd".to_string());
        task.args = Some(vec![
            "/C".to_string(),
            "echo %CD% & echo %BEE_TEST_VAR%".to_string(),
        ]);
    } else {
        task.program = Some("sh".to_string());
        task.args = Some(vec![
            "-c".to_string(),
            "pwd; printf '%s' \"$BEE_TEST_VAR\"".to_string(),
        ]);
    }

    let result = execute_task(&task).unwrap();
    assert!(result.success);
    assert!(result.stdout.contains("bee-value"));
    let normalized_out = result.stdout.replace('\\', "/");
    let normalized_dir = dir.to_string_lossy().replace('\\', "/");
    assert!(
        normalized_out.contains(&normalized_dir)
            || normalized_out.contains(dir.file_name().unwrap().to_str().unwrap())
    );
}

#[test]
fn process_executor_captures_stderr_and_exit_code() {
    let mut task = base_task("fail");
    task.executor = Some("process".to_string());
    if cfg!(windows) {
        task.program = Some("cmd".to_string());
        task.args = Some(vec![
            "/C".to_string(),
            "echo err 1>&2 & exit /B 7".to_string(),
        ]);
    } else {
        task.program = Some("sh".to_string());
        task.args = Some(vec![
            "-c".to_string(),
            "printf err 1>&2; exit 7".to_string(),
        ]);
    }

    let result = execute_task(&task).unwrap();
    assert!(!result.success);
    assert_eq!(result.exit_code, 7);
    assert!(result.stderr.contains("err"));
}

#[test]
fn missing_program_returns_error() {
    let mut task = base_task("missing");
    task.executor = Some("process".to_string());
    task.program = Some("definitely-not-a-real-bee-program-12345".to_string());
    let err = execute_task(&task).unwrap_err();
    match err {
        ExecutionError::ProgramNotFound { program, .. } => {
            assert!(program.contains("definitely-not-a-real-bee-program"))
        }
        other => panic!("unexpected error: {}", other),
    }
}

#[test]
fn shell_executor_runs_command() {
    let mut task = base_task("shell");
    task.executor = Some("shell".to_string());
    if cfg!(windows) {
        task.shell = Some("cmd".to_string());
        task.run = Some("echo shell-ok".to_string());
    } else {
        task.shell = Some("sh".to_string());
        task.run = Some("printf shell-ok".to_string());
    }
    let result = execute_task(&task).unwrap();
    assert!(result.success);
    assert!(result.stdout.contains("shell-ok"));
}

#[test]
fn script_executor_runs_python_when_available() {
    let interpreter = if cfg!(windows) { "python" } else { "python3" };
    if !bee::execution::shell::program_exists(interpreter) {
        eprintln!("skipping python script test: {} not found", interpreter);
        return;
    }
    let dir = temp_dir("script");
    let script = dir.join("script.py");
    fs::write(&script, "import sys\nprint('py-' + sys.argv[1])\n").unwrap();

    let mut task = base_task("script");
    task.executor = Some("script".to_string());
    task.interpreter = Some(interpreter.to_string());
    task.script = Some(script.to_string_lossy().to_string());
    task.args = Some(vec!["ok".to_string()]);

    let result = execute_task(&task).unwrap();
    assert!(result.success);
    assert!(result.stdout.contains("py-ok"));
}
