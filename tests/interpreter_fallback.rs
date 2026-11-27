use bee::execution::platform::resolve_task_for_platform;
use bee::execution::{
    execute_task, program_exists, resolve_interpreter, resolve_interpreter_for_task, ExecutionError,
};
use bee::parser::{InterpreterSpec, PlatformOverride, Task};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static COUNTER: AtomicUsize = AtomicUsize::new(0);
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "bee-interpreter-{}-{}-{}-{}",
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

fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

/// YAML-safe path spelling (forward slashes work on Windows too).
fn yaml_path(path: &Path) -> String {
    path_string(path).replace('\\', "/")
}

/// A tiny "interpreter" that ignores its arguments and prints `marker`.
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

/// Prints the second argument. The script executor prepends the script path,
/// so the second argument is the first configured task argument.
fn echo_second_arg_program(dir: &Path, stem: &str) -> PathBuf {
    #[cfg(windows)]
    {
        let path = dir.join(format!("{}.cmd", stem));
        fs::write(&path, "@echo off\r\necho arg=%2\r\n").unwrap();
        path
    }
    #[cfg(not(windows))]
    {
        let path = dir.join(stem);
        fs::write(&path, "#!/bin/sh\nprintf 'arg=%s' \"$2\"\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
}

fn script_task(name: &str, script: Option<String>, candidates: Vec<String>) -> Task {
    Task {
        name: name.to_string(),
        executor: Some("script".to_string()),
        interpreter_candidates: Some(candidates),
        script,
        ..Task::default()
    }
}

/// Freshly written executables can briefly report ETXTBSY on busy CI runners.
fn run_task_with_retry(task: &Task) -> Result<bee::execution::ExecutionResult, ExecutionError> {
    for attempt in 0..5 {
        match execute_task(task) {
            Err(ExecutionError::Spawn { source, .. })
                if source.raw_os_error() == Some(26) && attempt < 4 =>
            {
                thread::sleep(Duration::from_millis(20));
            }
            result => return result,
        }
    }
    unreachable!("retry loop always returns")
}

#[test]
fn parses_interpreter_list_and_roundtrips() {
    let yaml = r#"
name: report
executor: script
interpreter:
  - python3
  - python
  - py
script: scripts/report.py
args:
  - --format
  - json
"#;
    let task: Task = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(task.executor.as_deref(), Some("script"));
    assert_eq!(task.interpreter, None);
    assert_eq!(
        task.interpreter_candidates.as_deref(),
        Some(
            &[
                "python3".to_string(),
                "python".to_string(),
                "py".to_string()
            ][..]
        )
    );
    assert_eq!(task.effective_executor(), "script");
    assert_eq!(
        task.interpreter_candidates(),
        vec!["python3", "python", "py"]
    );
    task.validate().unwrap();

    let spec = task.interpreter_spec().unwrap();
    assert_eq!(
        spec,
        InterpreterSpec::List(vec![
            "python3".to_string(),
            "python".to_string(),
            "py".to_string(),
        ])
    );
    assert_eq!(spec.as_slice()[0], "python3");
    assert_eq!(spec.into_vec().len(), 3);

    let serialized = serde_yaml::to_string(&task).unwrap();
    assert!(serialized.contains("interpreter:"), "{}", serialized);
    assert!(serialized.contains("python3"), "{}", serialized);
    let reparsed: Task = serde_yaml::from_str(&serialized).unwrap();
    assert_eq!(task, reparsed);
}

#[test]
fn single_interpreter_string_is_preserved_and_runs() {
    let dir = temp_dir("single");
    let interpreter = marker_program(&dir, "single-interpreter", "single-ok");
    let script = dir.join("script.py");
    fs::write(&script, "pass\n").unwrap();

    let yaml = format!(
        "name: single\nexecutor: script\ninterpreter: {}\nscript: {}\n",
        yaml_path(&interpreter),
        yaml_path(&script)
    );
    let task: Task = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(
        task.interpreter
            .as_deref()
            .map(|value| value.replace('\\', "/")),
        Some(yaml_path(&interpreter))
    );
    assert_eq!(task.interpreter_candidates, None);
    assert_eq!(
        task.interpreter_candidates()
            .iter()
            .map(|value| value.replace('\\', "/"))
            .collect::<Vec<_>>(),
        vec![yaml_path(&interpreter)]
    );
    task.validate().unwrap();

    let result = run_task_with_retry(&task).unwrap();
    assert!(result.success, "stderr: {}", result.stderr);
    assert!(result.stdout.contains("single-ok"));
}

#[test]
fn build_backend_config_runs_with_interpreter_list() {
    let has_python = ["python3", "python", "py"].into_iter().any(program_exists);
    if !has_python {
        eprintln!("skipping: no python interpreter found on PATH");
        return;
    }

    let dir = temp_dir("build-backend");
    fs::create_dir_all(dir.join("scripts/maintenance")).unwrap();
    let script = dir.join("scripts/maintenance/build.py");
    fs::write(&script, "import sys\nprint('backend-ok:' + sys.argv[1])\n").unwrap();

    let yaml = format!(
        "name: build-backend\nexecutor: script\ninterpreter:\n  - python3\n  - python\n  - py\nargs:\n  - backend\nscript: {}\n",
        yaml_path(&script)
    );
    let task = bee::parser::reader::config::read_task_from_string(&yaml, "build-backend").unwrap();
    assert_eq!(
        task.interpreter_candidates(),
        vec!["python3", "python", "py"]
    );

    let result = run_task_with_retry(&task).unwrap();
    assert!(result.success, "stderr: {}", result.stderr);
    assert!(
        result.stdout.contains("backend-ok:backend"),
        "stdout: {}",
        result.stdout
    );
}

#[test]
fn first_available_interpreter_is_selected() {
    let dir = temp_dir("first");
    let first = marker_program(&dir, "first", "first-ok");
    let second = marker_program(&dir, "second", "second-ok");
    let script = dir.join("script.py");
    fs::write(&script, "pass\n").unwrap();

    let task = script_task(
        "first-available",
        Some(path_string(&script)),
        vec![path_string(&first), path_string(&second)],
    );
    let selected = resolve_interpreter_for_task(&task).unwrap();
    assert_eq!(selected, path_string(&first));

    let result = run_task_with_retry(&task).unwrap();
    assert!(result.success, "stderr: {}", result.stderr);
    assert!(result.stdout.contains("first-ok"));
    assert!(!result.stdout.contains("second-ok"));
}

#[test]
fn falls_back_to_second_interpreter() {
    let dir = temp_dir("fallback");
    let second = marker_program(&dir, "second", "second-ok");
    let script = dir.join("script.py");
    fs::write(&script, "pass\n").unwrap();

    let task = script_task(
        "fallback",
        Some(path_string(&script)),
        vec![missing_candidate("first"), path_string(&second)],
    );
    let selected = resolve_interpreter(&task.name, &task.interpreter_candidates()).unwrap();
    assert_eq!(selected, path_string(&second));

    let result = run_task_with_retry(&task).unwrap();
    assert!(result.success, "stderr: {}", result.stderr);
    assert!(result.stdout.contains("second-ok"));
}

#[test]
fn no_fallback_after_selected_interpreter_fails() {
    let dir = temp_dir("no-retry");
    let failing = failing_program(&dir, "failing");
    let good = marker_program(&dir, "good", "should-not-run");
    let script = dir.join("script.py");
    fs::write(&script, "pass\n").unwrap();

    let task = script_task(
        "no-retry",
        Some(path_string(&script)),
        vec![path_string(&failing), path_string(&good)],
    );

    let result = run_task_with_retry(&task).unwrap();
    assert!(!result.success);
    assert_eq!(result.exit_code, 7);
    assert!(!result.stdout.contains("should-not-run"));
}

#[test]
fn no_available_interpreter_lists_all_attempts() {
    let script = temp_dir("none").join("script.py");
    let candidates = vec![missing_candidate("a"), missing_candidate("b")];
    let task = script_task("none", Some(path_string(&script)), candidates.clone());

    let err = run_task_with_retry(&task).unwrap_err();
    let message = err.to_string();
    assert!(message.contains("interpreter"), "message: {}", message);
    assert!(message.contains(&candidates[0]), "message: {}", message);
    assert!(message.contains(&candidates[1]), "message: {}", message);
    match err {
        ExecutionError::InterpreterNotFound {
            candidates: attempted,
            ..
        } => {
            assert_eq!(attempted, candidates)
        }
        other => panic!("unexpected error: {}", other),
    }
}

#[test]
fn empty_and_blank_interpreter_lists_are_rejected() {
    let empty: Task =
        serde_yaml::from_str("name: empty\nexecutor: script\ninterpreter: []\nscript: script.py\n")
            .unwrap();
    let err = empty.validate().unwrap_err().to_string();
    assert!(err.contains("empty"), "error: {}", err);
    assert!(err.contains("interpreter"), "error: {}", err);
    let err = execute_task(&empty).unwrap_err().to_string();
    assert!(err.contains("empty"), "error: {}", err);

    let blank: Task = serde_yaml::from_str(
        "name: blank\nexecutor: script\ninterpreter: ['  ']\nscript: script.py\n",
    )
    .unwrap();
    let err = blank.validate().unwrap_err().to_string();
    assert!(err.contains("empty"), "error: {}", err);
}

#[test]
fn arguments_are_passed_to_selected_interpreter() {
    let dir = temp_dir("args");
    let interpreter = echo_second_arg_program(&dir, "echo-args");
    let script = dir.join("script.py");
    fs::write(&script, "pass\n").unwrap();

    let mut task = script_task(
        "args",
        Some(path_string(&script)),
        vec![missing_candidate("args"), path_string(&interpreter)],
    );
    task.args = Some(vec!["payload".to_string()]);

    let result = run_task_with_retry(&task).unwrap();
    assert!(result.success, "stderr: {}", result.stderr);
    assert!(
        result.stdout.contains("arg=payload"),
        "stdout: {}",
        result.stdout
    );
}

#[test]
fn platform_override_can_provide_interpreter_list() {
    let dir = temp_dir("platform");
    let interpreter = marker_program(&dir, "platform-interpreter", "platform-ok");
    let script = dir.join("script.py");
    fs::write(&script, "pass\n").unwrap();

    let mut platforms = BTreeMap::new();
    platforms.insert(
        std::env::consts::OS.to_string(),
        PlatformOverride {
            interpreter_candidates: Some(vec![
                missing_candidate("platform"),
                path_string(&interpreter),
            ]),
            ..PlatformOverride::default()
        },
    );
    let task = Task {
        name: "platform".to_string(),
        executor: Some("script".to_string()),
        interpreter: Some("unused-interpreter".to_string()),
        script: Some(path_string(&script)),
        platforms: Some(platforms),
        ..Task::default()
    };
    task.validate().unwrap();

    let expected = task
        .platforms
        .as_ref()
        .unwrap()
        .get(std::env::consts::OS)
        .unwrap()
        .interpreter_candidates
        .clone()
        .unwrap();
    let resolved = resolve_task_for_platform(&task).unwrap();
    assert_eq!(resolved.interpreter_candidates(), expected);

    let result = run_task_with_retry(&task).unwrap();
    assert!(result.success, "stderr: {}", result.stderr);
    assert!(result.stdout.contains("platform-ok"));
}

#[test]
fn executor_list_is_rejected() {
    let err = serde_yaml::from_str::<Task>(
        "name: bad\nexecutor:\n  - python3\n  - python\nscript: script.py\n",
    )
    .unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains("sequence") || message.contains("string"),
        "message: {}",
        message
    );
}

#[cfg(windows)]
#[test]
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
    let selected = resolve_interpreter("win-ext", &[missing_candidate("x"), "bee-tool".into()]);
    assert_eq!(selected.unwrap(), "bee-tool");
    match old_path {
        Some(path) => std::env::set_var("PATH", path),
        None => std::env::remove_var("PATH"),
    }
}

#[cfg(not(windows))]
#[test]
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
    let selected = resolve_interpreter("pathext", &["bee-tool".to_string()]).unwrap();
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
