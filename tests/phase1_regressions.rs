//! Regression tests for Phase 1: exit codes, rules, cache, DAG, backup and
//! path safety. Each test runs the real CLI binary in an isolated directory.

use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};

struct Project {
    dir: tempfile::TempDir,
}

impl Project {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        bee(dir.path()).args(["init", "--yes"]).assert().success();
        Self { dir }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.dir.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.dir.path().join(relative)).unwrap()
    }
}

fn bee(dir: &Path) -> Command {
    let mut command = Command::cargo_bin("bee").unwrap();
    command.current_dir(dir);
    command
}

fn stderr_of(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).to_string()
}

fn stdout_of(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stdout).to_string()
}

#[test]
fn failed_task_preserves_its_real_exit_code() {
    let project = Project::new();
    project.write(
        "bee/tasks/build.yml",
        "name: build\nexecutor: shell\nrun: exit 7\n",
    );

    let assert = bee(project.path())
        .args(["task", "run", "build"])
        .assert()
        .failure()
        .code(7);
    assert!(stderr_of(&assert).contains("exit code 7"));
}

#[test]
fn unknown_yaml_field_is_rejected() {
    let project = Project::new();
    project.write(
        "bee/tasks/build.yml",
        "name: build\nrun: echo hi\ntypo_field: 1\n",
    );

    let assert = bee(project.path())
        .args(["task", "run", "build"])
        .assert()
        .failure();
    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("typo_field") || stderr.contains("unknown field"),
        "unexpected error: {}",
        stderr
    );
}

#[test]
fn manual_rule_skips_task_unless_force_is_given() {
    let project = Project::new();
    bee(project.path())
        .args(["rule", "create", "manual-build", "--task", "build"])
        .assert()
        .success();

    let skipped = bee(project.path())
        .args(["task", "run", "build"])
        .assert()
        .success();
    assert!(
        stdout_of(&skipped).contains("skipped"),
        "expected skip, got: {}",
        stdout_of(&skipped)
    );
    assert!(!project.path().join("bee/logs/build.log").exists());

    let forced = bee(project.path())
        .args(["task", "run", "build", "--force"])
        .assert()
        .success();
    assert!(
        stdout_of(&forced).contains("running"),
        "expected run, got: {}",
        stdout_of(&forced)
    );
    assert!(project.read("bee/logs/build.log").contains("Building"));
}

#[test]
fn failed_task_retries_because_trigger_state_was_not_committed() {
    let project = Project::new();
    project.write("src/data.txt", "v1");
    bee(project.path())
        .args(["rule", "create", "on-change", "--task", "build"])
        .assert()
        .success();
    bee(project.path())
        .args([
            "rule",
            "modify",
            "on-change",
            "--trigger-type",
            "file_change",
            "--trigger-param",
            "paths=src/data.txt",
        ])
        .assert()
        .success();

    // Cold cache: the first run establishes a baseline and skips the task.
    bee(project.path())
        .args(["task", "run", "build"])
        .assert()
        .success();

    project.write("src/data.txt", "v2");
    project.write(
        "bee/tasks/build.yml",
        "name: build\nexecutor: shell\nrun: exit 7\n",
    );

    // The trigger fires twice in a row: the failure must not consume it.
    bee(project.path())
        .args(["task", "run", "build"])
        .assert()
        .failure()
        .code(7);
    bee(project.path())
        .args(["task", "run", "build"])
        .assert()
        .failure()
        .code(7);
}

#[test]
fn unknown_dependency_is_an_error() {
    let project = Project::new();
    project.write(
        "bee/tasks/build.yml",
        "name: build\ndepends_on:\n  - ghost\nrun: echo hi\n",
    );

    let assert = bee(project.path())
        .args(["pipeline", "run", "main"])
        .assert()
        .failure();
    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("ghost"),
        "expected unknown dependency error, got: {}",
        stderr
    );
}

#[test]
fn backup_hash_tracks_content_and_restore_is_exact() {
    let project = Project::new();
    bee(project.path())
        .args(["backup", "create"])
        .assert()
        .success();
    let manifest: json::JsonValue =
        json::parse(&project.read("bee/system/backup/.manifest.json")).expect("valid manifest");
    let first_hash = manifest["versions"][0]["hash"]
        .as_str()
        .expect("backup hash")
        .to_string();

    project.write(
        "bee/tasks/build.yml",
        "name: build\nrun: echo DIFFERENT CONTENT\n",
    );
    bee(project.path())
        .args(["backup", "create"])
        .assert()
        .success();
    let second: json::JsonValue =
        json::parse(&project.read("bee/system/backup/.manifest.json")).expect("valid manifest");
    let second_hash = second["versions"][1]["hash"]
        .as_str()
        .expect("second backup hash")
        .to_string();
    assert_ne!(
        first_hash, second_hash,
        "content change must change the hash"
    );

    bee(project.path())
        .args(["backup", "restore", &first_hash])
        .assert()
        .success();
    assert!(
        project.read("bee/tasks/build.yml").contains("Building"),
        "restore did not bring back the original task body"
    );
}

#[test]
fn unsafe_names_are_rejected_everywhere() {
    let project = Project::new();
    for args in [
        vec!["task", "run", "../evil"],
        vec!["task", "create", "../evil"],
        vec!["pipeline", "create", "../evil"],
        vec!["rule", "create", "../evil", "--task", "build"],
        vec!["backup", "restore", "../evil"],
    ] {
        bee(project.path()).args(args.clone()).assert().failure();
    }
}

#[test]
fn init_does_not_wipe_an_existing_bee_directory_without_consent() {
    let dir = tempfile::tempdir().unwrap();
    let bee_dir = dir.path().join("bee");
    fs::create_dir_all(&bee_dir).unwrap();
    fs::write(bee_dir.join("sentinel"), "keep me").unwrap();

    bee(dir.path())
        .args(["init"])
        .write_stdin("n\n")
        .assert()
        .failure();
    assert!(
        bee_dir.join("sentinel").exists(),
        "file was deleted without consent"
    );

    bee(dir.path()).args(["init", "--yes"]).assert().success();
    assert!(
        !bee_dir.join("sentinel").exists(),
        "--yes should replace the existing directory"
    );
}

#[test]
fn validated_names_reject_traversal() {
    assert!(bee::safety::validate_name("task", "build").is_ok());
    assert!(bee::safety::validate_name("task", "../evil").is_err());
    assert!(bee::safety::validate_name("task", "").is_err());
    let path = bee::safety::safe_path("bee/tasks", "task", "build").unwrap();
    assert_eq!(path, PathBuf::from("bee/tasks/build"));
}
