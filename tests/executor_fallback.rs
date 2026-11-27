use bee::execution::{execute_task, program_exists, resolve_executor, ExecutionError};
use bee::parser::{ExecutionMode, ExecutorSpec, PlatformOverride, Task};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicUsize = AtomicUsize::new(0);
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "bee-fallback-{}-{}-{}-{}",
        name,
        std::process::id(),
        nanos,
        seq
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn missing_candidate(label: &str) -> String {
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("bee-missing-{}-{}", label, seq)
}

/// Create a small executable script/batch file that prints `marker`.
fn marker_program(dir: &Path, stem: &str, marker: &str) -> PathBuf {
    #[cfg(windows)]
    {
        let path = dir.join(format!("{}.cmd", stem));
        fs::write(&path, format!("@echo off\r\necho {}\r\n", marker)).unwrap();
        path
    }
    #[cfg(not(windows))]
    {
        let path = dir.join(stem);
        fs::write(&path, format!("#!/bin/sh\nprintf '%s' '{}'\n", marker)).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
}

fn failing_program(dir: &Path, stem: &str) -> PathBuf {
    #[cfg(windows)]
    {
        let path = dir.join(format!("{}.cmd", stem));
        fs::write(&path, "@echo off\r\nexit /B 7\r\n").unwrap();
        path
    }
    #[cfg(not(windows))]
    {
        let path = dir.join(stem);
        fs::write(&path, "#!/bin/sh\nexit 7\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
}

fn available_python() -> Option<&'static str> {
    ["python3", "python", "py"]
        .into_iter()
        .find(|candidate| program_exists(candidate))
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[test]
fn parses_executor_list_and_roundtrips() {
    let yaml = r#"
name: report
executor:
  - python3
  - python
args:
  - -c
  - print('ok')
"#;
    let task: Task = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(task.executor, None);
    assert_eq!(
        task.executor_candidates.as_deref(),
        Some(&["python3".to_string(), "python".to_string()][..])
    );
    assert_eq!(task.execution_mode(), ExecutionMode::Executable);
    assert_eq!(
        task.executor_spec(),
        Some(ExecutorSpec::List(vec![
            "python3".to_string(),
            "python".to_string()
        ]))
    );
    task.validate().unwrap();

    let serialized = serde_yaml::to_string(&task).unwrap();
    assert!(serialized.contains("executor:"));
    assert!(serialized.contains("python3"));
    let reparsed: Task = serde_yaml::from_str(&serialized).unwrap();
    assert_eq!(task, reparsed);
}

#[test]
fn parses_existing_string_executor_forms() {
    let process: Task =
        serde_yaml::from_str("name: build\nexecutor: process\nprogram: cargo\nargs: [build]\n")
            .unwrap();
    assert_eq!(process.effective_executor(), "process");
    assert_eq!(process.execution_mode(), ExecutionMode::Process);
    assert_eq!(process.executor_candidates(), Vec::<String>::new());

    let legacy: Task = serde_yaml::from_str("name: old\nrun: echo hi\n").unwrap();
    assert_eq!(legacy.effective_executor(), "legacy");
    assert_eq!(legacy.execution_mode(), ExecutionMode::Legacy);
}

#[test]
fn single_executable_executor_runs_python() {
    let Some(python) = available_python() else {
        eprintln!("skipping: no python interpreter found on PATH");
        return;
    };

    let task = Task {
        name: "single-python".to_string(),
        executor: Some(python.to_string()),
        args: Some(vec![
            "-c".to_string(),
            "print('single-python-ok')".to_string(),
        ]),
        ..Task::default()
    };
    task.validate().unwrap();
    let result = execute_task(&task).unwrap();
    assert!(result.success, "stderr: {}", result.stderr);
    assert!(result.stdout.contains("single-python-ok"));
}

#[test]
fn executor_list_selects_first_available() {
    let dir = temp_dir("first");
    let first = marker_program(&dir, "first", "first-ok");
    let second = marker_program(&dir, "second", "second-ok");
    let task = Task {
        name: "first-available".to_string(),
        executor_candidates: Some(vec![path_string(&first), path_string(&second)]),
        ..Task::default()
    };

    let result = execute_task(&task).unwrap();
    assert!(result.success, "stderr: {}", result.stderr);
    assert!(result.stdout.contains("first-ok"));
    assert!(!result.stdout.contains("second-ok"));
}

#[test]
fn executor_list_falls_back_to_second_candidate() {
    let dir = temp_dir("fallback");
    let second = marker_program(&dir, "second", "second-ok");
    let candidates = vec![missing_candidate("first"), path_string(&second)];
    let task = Task {
        name: "fallback".to_string(),
        executor_candidates: Some(candidates),
        ..Task::default()
    };

    let selected = resolve_executor(&task.name, &task.executor_candidates()).unwrap();
    assert_eq!(selected, path_string(&second));

    let result = execute_task(&task).unwrap();
    assert!(result.success, "stderr: {}", result.stderr);
    assert!(result.stdout.contains("second-ok"));
}

#[test]
fn no_fallback_after_selected_executable_fails() {
    let dir = temp_dir("no-retry");
    let failing = failing_program(&dir, "failing");
    let good = marker_program(&dir, "good", "should-not-run");
    let task = Task {
        name: "no-retry".to_string(),
        executor_candidates: Some(vec![path_string(&failing), path_string(&good)]),
        ..Task::default()
    };

    let result = execute_task(&task).unwrap();
    assert!(!result.success);
    assert_eq!(result.exit_code, 7);
    assert!(!result.stdout.contains("should-not-run"));
}

#[test]
fn no_available_executor_lists_all_attempts() {
    let candidates = vec![missing_candidate("a"), missing_candidate("b")];
    let task = Task {
        name: "none".to_string(),
        executor_candidates: Some(candidates.clone()),
        args: Some(vec!["--version".to_string()]),
        ..Task::default()
    };

    let err = execute_task(&task).unwrap_err();
    let message = err.to_string();
    assert!(message.contains("executor"), "message: {}", message);
    assert!(message.contains(&candidates[0]), "message: {}", message);
    assert!(message.contains(&candidates[1]), "message: {}", message);
    match err {
        ExecutionError::ExecutorNotFound {
            candidates: attempted,
            ..
        } => assert_eq!(attempted, candidates),
        other => panic!("unexpected error: {}", other),
    }
}

#[test]
fn empty_executor_list_is_rejected() {
    let task: Task = serde_yaml::from_str("name: empty\nexecutor: []\n").unwrap();
    let err = task.validate().unwrap_err().to_string();
    assert!(err.contains("empty"), "error: {}", err);
    assert!(err.contains("executor"), "error: {}", err);

    let err = execute_task(&task).unwrap_err().to_string();
    assert!(err.contains("empty"), "error: {}", err);
}

#[test]
fn blank_executor_candidate_is_rejected() {
    let task: Task = serde_yaml::from_str("name: blank\nexecutor: ['  ']\n").unwrap();
    let err = task.validate().unwrap_err().to_string();
    assert!(err.contains("empty"), "error: {}", err);
}

#[test]
fn arguments_are_passed_to_selected_executable() {
    let dir = temp_dir("args");
    #[cfg(windows)]
    let program = {
        let path = dir.join("args.cmd");
        fs::write(&path, "@echo off\r\necho arg=%1\r\n").unwrap();
        path
    };
    #[cfg(not(windows))]
    let program = {
        let path = dir.join("args");
        fs::write(&path, "#!/bin/sh\nprintf 'arg=%s' \"$1\"\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    };

    let task = Task {
        name: "args".to_string(),
        executor_candidates: Some(vec![missing_candidate("args"), path_string(&program)]),
        args: Some(vec!["a b".to_string()]),
        ..Task::default()
    };

    let result = execute_task(&task).unwrap();
    assert!(result.success, "stderr: {}", result.stderr);
    assert!(result.stdout.contains("a b"), "stdout: {}", result.stdout);
}

#[test]
fn executor_list_can_provide_script_interpreter_fallback() {
    let Some(python) = available_python() else {
        eprintln!("skipping: no python interpreter found on PATH");
        return;
    };
    let dir = temp_dir("script");
    let script = dir.join("script.py");
    fs::write(&script, "import sys\nprint('py-' + sys.argv[1])\n").unwrap();

    let task = Task {
        name: "script".to_string(),
        executor_candidates: Some(vec![missing_candidate("python"), python.to_string()]),
        script: Some(path_string(&script)),
        args: Some(vec!["ok".to_string()]),
        ..Task::default()
    };

    let result = execute_task(&task).unwrap();
    assert!(result.success, "stderr: {}", result.stderr);
    assert!(result.stdout.contains("py-ok"), "stdout: {}", result.stdout);
}

#[test]
fn executor_list_can_provide_shell_fallback() {
    let shell = if cfg!(windows) { "cmd" } else { "sh" };
    let command = if cfg!(windows) {
        "echo shell-fallback-ok"
    } else {
        "printf shell-fallback-ok"
    };
    let task = Task {
        name: "shell".to_string(),
        executor_candidates: Some(vec![missing_candidate("shell"), shell.to_string()]),
        run: Some(command.to_string()),
        ..Task::default()
    };

    let result = execute_task(&task).unwrap();
    assert!(result.success, "stderr: {}", result.stderr);
    assert!(result.stdout.contains("shell-fallback-ok"));
}

#[test]
fn platform_override_can_use_executor_list() {
    let dir = temp_dir("platform");
    let program = marker_program(&dir, "platform", "platform-ok");
    let mut platforms = BTreeMap::new();
    platforms.insert(
        std::env::consts::OS.to_string(),
        PlatformOverride {
            executor_candidates: Some(vec![missing_candidate("platform"), path_string(&program)]),
            ..PlatformOverride::default()
        },
    );
    let task = Task {
        name: "platform".to_string(),
        platforms: Some(platforms),
        ..Task::default()
    };

    let result = execute_task(&task).unwrap();
    assert!(result.success, "stderr: {}", result.stderr);
    assert!(result.stdout.contains("platform-ok"));
}

#[test]
#[cfg(windows)]
fn windows_executable_extensions_are_resolved() {
    let dir = temp_dir("win-ext");
    let script = dir.join("bee-tool.cmd");
    fs::write(&script, "@echo off\r\necho win-ext-ok\r\n").unwrap();

    // A path without an extension must find the `.cmd` file via PATHEXT.
    let no_extension = dir.join("bee-tool");
    assert!(program_exists(&path_string(&no_extension)));

    let _guard = ENV_LOCK.lock().unwrap();
    let old_path = std::env::var_os("PATH");
    let mut paths = vec![dir.clone()];
    if let Some(old) = &old_path {
        paths.extend(std::env::split_paths(old));
    }
    std::env::set_var("PATH", std::env::join_paths(paths).unwrap());
    assert!(program_exists("bee-tool"));
    let selected = resolve_executor("win-ext", &[missing_candidate("x"), "bee-tool".into()]);
    assert_eq!(selected.unwrap(), "bee-tool");
    match old_path {
        Some(path) => std::env::set_var("PATH", path),
        None => std::env::remove_var("PATH"),
    }
}

#[test]
#[cfg(not(windows))]
fn pathext_resolution_works_when_configured() {
    let dir = temp_dir("pathext");
    fs::write(dir.join("bee-tool.exe"), b"not a real windows binary\n").unwrap();

    let _guard = ENV_LOCK.lock().unwrap();
    let old_path = std::env::var_os("PATH");
    let old_pathext = std::env::var_os("PATHEXT");
    let mut paths = vec![dir];
    if let Some(old) = &old_path {
        paths.extend(std::env::split_paths(old));
    }
    std::env::set_var("PATH", std::env::join_paths(paths).unwrap());
    std::env::set_var("PATHEXT", ".EXE;.CMD");

    assert!(program_exists("bee-tool"));
    let selected = resolve_executor("pathext", &["bee-tool".to_string()]).unwrap();
    assert_eq!(selected, "bee-tool");

    match old_path {
        Some(path) => std::env::set_var("PATH", path),
        None => std::env::remove_var("PATH"),
    }
    match old_pathext {
        Some(path) => std::env::set_var("PATHEXT", path),
        None => std::env::remove_var("PATHEXT"),
    }
}
