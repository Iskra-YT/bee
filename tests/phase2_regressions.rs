//! Regression tests for Phase 2: execution timeout, environment isolation,
//! log format and working directory handling.

use assert_cmd::Command;
use std::fs;
use std::time::{Duration, Instant};

struct Project {
    dir: tempfile::TempDir,
}

impl Project {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        bee(dir.path()).args(["init", "--yes"]).assert().success();
        Self { dir }
    }

    fn path(&self) -> &std::path::Path {
        self.dir.path()
    }

    fn write_task(&self, yaml: &str) {
        fs::write(self.dir.path().join("bee/tasks/build.yml"), yaml).unwrap();
    }

    fn log(&self) -> String {
        fs::read_to_string(self.dir.path().join("bee/logs/build.log")).unwrap()
    }
}

fn bee(dir: &std::path::Path) -> Command {
    let mut command = Command::cargo_bin("bee").unwrap();
    command.current_dir(dir);
    command
}

#[test]
fn timeout_kills_a_running_task_and_fails_fast() {
    let project = Project::new();
    let run = if cfg!(windows) {
        // Keep the loop inside cmd.exe: a child process would inherit the
        // output pipes and keep them open after the timeout kills cmd.exe.
        "for /L %i in (1,1,2147483647) do @rem"
    } else {
        "while true; do :; done"
    };
    project.write_task(&format!(
        "name: build\nexecutor: shell\nrun: \"{}\"\ntimeout: 1s\n",
        run
    ));

    let started = Instant::now();
    let assert = bee(project.path())
        .args(["task", "run", "build"])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("timed out"), "unexpected error: {}", stderr);
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "timeout did not kill the process promptly"
    );
}

#[test]
fn inherit_env_false_drops_unrelated_variables() {
    let project = Project::new();
    let run = if cfg!(windows) {
        "echo %BEE_TEST_VAR%"
    } else {
        "printf \"%s\" \"$BEE_TEST_VAR\""
    };
    project.write_task(&format!("name: build\nexecutor: shell\nrun: '{}'\n", run));
    bee(project.path())
        .env("BEE_TEST_VAR", "visible-value")
        .args(["task", "run", "build"])
        .assert()
        .success();
    assert!(project.log().contains("visible-value"));

    project.write_task(&format!(
        "name: build\nexecutor: shell\nrun: '{}'\ninherit_env: false\n",
        run
    ));
    bee(project.path())
        .env("BEE_TEST_VAR", "visible-value")
        .args(["task", "run", "build"])
        .assert()
        .success();
    assert!(
        !project.log().contains("visible-value"),
        "inherit_env: false leaked a process variable: {}",
        project.log()
    );
}

#[test]
fn logs_include_exit_code_and_redact_secrets() {
    let project = Project::new();
    project.write_task(
        "name: build\nexecutor: shell\nrun: 'printf \"%s\" \"supersecrettoken123\"'\nenv:\n  MY_TOKEN: supersecrettoken123\n",
    );
    bee(project.path())
        .args(["task", "run", "build"])
        .assert()
        .success();

    let log = project.log();
    assert!(log.contains("exit code: 0"), "log: {}", log);
    assert!(log.contains("[stdout]"), "log: {}", log);
    assert!(log.contains("[stderr]"), "log: {}", log);
    assert!(
        !log.contains("supersecrettoken123"),
        "secret leaked into log: {}",
        log
    );
    assert!(log.contains("[REDACTED]"), "log: {}", log);
}

#[test]
fn relative_working_dir_is_anchored_to_project_root() {
    let project = Project::new();
    fs::create_dir_all(project.path().join("nested")).unwrap();
    project.write_task("name: build\nexecutor: shell\nrun: echo ok\nworking_dir: nested\n");
    bee(project.path())
        .args(["task", "run", "build"])
        .assert()
        .success();
    assert!(project.log().contains("ok"));
}

#[test]
fn timeout_values_are_parsed() {
    use bee::time::parse_duration;
    assert_eq!(parse_duration("30").unwrap(), Duration::from_secs(30));
    assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
    assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
    assert!(parse_duration("0s").is_err());
    assert!(parse_duration("nonsense").is_err());
}
