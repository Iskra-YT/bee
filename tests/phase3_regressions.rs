//! Regression tests for Phase 3: scheduler, inspect commands, global flags,
//! completions and fail-closed self-update helpers.

use assert_cmd::Command;
use std::fs;

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

    fn write(&self, relative: &str, content: &str) {
        let path = self.dir.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn exists(&self, relative: &str) -> bool {
        self.dir.path().join(relative).exists()
    }
}

fn bee(dir: &std::path::Path) -> Command {
    let mut command = Command::cargo_bin("bee").unwrap();
    command.current_dir(dir);
    command
}

fn stdout(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stdout).to_string()
}

fn stderr(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).to_string()
}

fn json_output(assert: &assert_cmd::assert::Assert) -> json::JsonValue {
    json::parse(&stdout(assert))
        .unwrap_or_else(|err| panic!("invalid JSON output '{}': {}", stdout(assert), err))
}

#[test]
fn schedule_list_and_once_daemon_fire_a_real_task() {
    let project = Project::new();
    project.write(
        "bee/tasks/build.yml",
        "name: build\nexecutor: shell\nrun: echo scheduled\n",
    );
    bee(project.path())
        .args(["rule", "create", "hourly", "--task", "build"])
        .assert()
        .success();
    bee(project.path())
        .args([
            "rule",
            "modify",
            "hourly",
            "--trigger-type",
            "schedule",
            "--trigger-param",
            "cron=* * * * *",
        ])
        .assert()
        .success();

    let listed = bee(project.path())
        .args(["--json", "schedule", "list"])
        .assert()
        .success();
    let data = json_output(&listed);
    assert_eq!(data["scheduled"][0]["cron"].as_str(), Some("* * * * *"));
    assert_eq!(data["scheduled"][0]["task"].as_str(), Some("build"));
    assert!(!data["scheduled"][0]["next_run"]
        .as_str()
        .unwrap()
        .is_empty());

    let dry = bee(project.path())
        .args(["--dry-run", "schedule", "run", "--once"])
        .assert()
        .success();
    assert!(
        stdout(&dry).contains("would run task 'build'"),
        "unexpected output: {}",
        stdout(&dry)
    );
    assert!(
        !project.exists("bee/logs/build.log"),
        "dry run executed the task"
    );

    let fired = bee(project.path())
        .args(["schedule", "run", "--once"])
        .assert()
        .success();
    assert!(
        stdout(&fired).contains("firing task 'build'"),
        "unexpected output: {}",
        stdout(&fired)
    );
    assert!(project.exists("bee/logs/build.log"));

    let again = bee(project.path())
        .args(["schedule", "run", "--once"])
        .assert()
        .success();
    assert!(
        !stdout(&again).contains("firing task"),
        "the same minute fired twice: {}",
        stdout(&again)
    );
}

#[test]
fn invalid_cron_expression_is_rejected() {
    let project = Project::new();
    bee(project.path())
        .args(["rule", "create", "broken", "--task", "build"])
        .assert()
        .success();
    let assert = bee(project.path())
        .args([
            "rule",
            "modify",
            "broken",
            "--trigger-type",
            "schedule",
            "--trigger-param",
            "cron=not a cron",
        ])
        .assert()
        .failure();
    assert!(
        stderr(&assert).contains("cron"),
        "unexpected error: {}",
        stderr(&assert)
    );
}

#[test]
fn validate_reports_bad_definitions_and_supports_doctor_alias_and_json() {
    let project = Project::new();
    bee(project.path()).args(["validate"]).assert().success();
    bee(project.path()).args(["doctor"]).assert().success();

    project.write(
        "bee/tasks/build.yml",
        "name: build\nrun: echo hi\nunknown_field: true\n",
    );
    let failed = bee(project.path()).args(["validate"]).assert().failure();
    assert!(
        stderr(&failed).contains("unknown_field"),
        "unexpected error: {}",
        stderr(&failed)
    );

    let report = bee(project.path())
        .args(["--json", "validate"])
        .assert()
        .failure();
    let data = json_output(&report);
    assert_eq!(data["ok"], false);
    assert!(data["checked"].as_usize().unwrap() > 0);
    assert!(!data["problems"].is_empty());
}

#[test]
fn logs_and_show_commands_read_only_configuration_and_logs() {
    let project = Project::new();
    bee(project.path())
        .args(["task", "run", "build", "--force"])
        .assert()
        .success();

    let list = bee(project.path())
        .args(["logs", "list"])
        .assert()
        .success();
    assert!(stdout(&list).contains("build"));

    let log = bee(project.path())
        .args(["logs", "task", "build"])
        .assert()
        .success();
    assert!(stdout(&log).contains("exit code: 0"));

    let json_log = bee(project.path())
        .args(["--json", "logs", "task", "build"])
        .assert()
        .success();
    let data = json_output(&json_log);
    assert_eq!(data["task"].as_str(), Some("build"));
    assert!(data["content"].as_str().unwrap().contains("[stdout]"));

    let shown = bee(project.path())
        .args(["show", "task", "build"])
        .assert()
        .success();
    assert!(stdout(&shown).contains("name: build"));

    let json_shown = bee(project.path())
        .args(["--json", "show", "pipeline", "main"])
        .assert()
        .success();
    let data = json_output(&json_shown);
    assert_eq!(data["kind"].as_str(), Some("pipeline"));
    assert_eq!(data["name"].as_str(), Some("main"));

    bee(project.path())
        .args(["logs", "task", "../evil"])
        .assert()
        .failure();
    bee(project.path())
        .args(["show", "task", "../evil"])
        .assert()
        .failure();
}

#[test]
fn json_list_and_status_are_machine_readable() {
    let project = Project::new();
    let listed = bee(project.path())
        .args(["--json", "list"])
        .assert()
        .success();
    let data = json_output(&listed);
    assert!(data["tasks"].is_array());
    assert!(data["pipelines"].is_array());
    assert!(data["rules"].is_array());

    let status = bee(project.path())
        .args(["--json", "status"])
        .assert()
        .success();
    let data = json_output(&status);
    assert_eq!(data["tasks"].as_usize().unwrap(), 2);
    assert_eq!(data["pipelines"].as_usize().unwrap(), 1);
}

#[test]
fn no_color_removes_ansi_and_dry_run_does_not_execute() {
    let project = Project::new();
    let colored = bee(project.path())
        .env_remove("NO_COLOR")
        .args(["validate"])
        .assert()
        .success();
    assert!(
        stdout(&colored).contains('\u{1b}'),
        "expected colored success output: {:?}",
        stdout(&colored)
    );

    let plain = bee(project.path())
        .env_remove("NO_COLOR")
        .args(["--no-color", "validate"])
        .assert()
        .success();
    assert!(
        !stdout(&plain).contains('\u{1b}'),
        "--no-color left ANSI codes: {:?}",
        stdout(&plain)
    );

    let planned = bee(project.path())
        .args(["--dry-run", "run"])
        .assert()
        .success();
    assert!(
        stdout(&planned).contains("[dry-run] pipeline 'main'"),
        "unexpected output: {}",
        stdout(&planned)
    );
    assert!(!project.exists("bee/logs/build.log"));
}

#[test]
fn completions_generate_real_shell_scripts() {
    let dir = tempfile::tempdir().unwrap();
    for shell in ["bash", "zsh", "fish", "powershell"] {
        let assert = bee(dir.path())
            .args(["completions", shell])
            .assert()
            .success();
        assert!(
            !stdout(&assert).trim().is_empty(),
            "empty completion script for {}",
            shell
        );
        assert!(
            stdout(&assert).contains("bee"),
            "completion script for {} does not mention bee",
            shell
        );
    }
    bee(dir.path())
        .args(["completions", "tcsh"])
        .assert()
        .failure();
}

#[cfg(unix)]
mod update_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    fn fake_bee(dir: &Path, name: &str, output: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        fs::write(&path, format!("#!/bin/sh\nprintf '%s\\n' '{}'\n", output)).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn smoketest_checks_the_reported_commit() {
        let dir = tempfile::tempdir().unwrap();
        let good = fake_bee(dir.path(), "good", SHA);
        bee::update::smoketest(&good, Some(SHA)).unwrap();
        assert!(bee::update::smoketest(&good, Some("f".repeat(40).as_str())).is_err());

        let silent = dir.path().join("silent");
        fs::write(
            &silent,
            "#!/bin/sh\nif [ \"$1\" = version ]; then exit 0; fi\n",
        )
        .unwrap();
        fs::set_permissions(&silent, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(bee::update::smoketest(&silent, None).is_err());
    }

    #[test]
    fn failed_post_swap_smoketest_rolls_back_the_previous_binary() {
        let dir = tempfile::tempdir().unwrap();
        let current = fake_bee(dir.path(), "bee", "OLD-BINARY");
        let new = fake_bee(dir.path(), "new", "NOT-A-RELEASE-BUILD");

        let err = bee::update::install_binary(&new, &current, Some(SHA)).unwrap_err();
        assert!(
            err.to_string().contains("smoke test"),
            "unexpected error: {}",
            err
        );
        let restored = fs::read_to_string(&current).unwrap();
        assert!(
            restored.contains("OLD-BINARY"),
            "previous binary was not restored: {}",
            restored
        );
    }

    #[test]
    fn verified_binary_is_installed_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let current = fake_bee(dir.path(), "bee", "OLD-BINARY");
        let new = fake_bee(dir.path(), "new", SHA);

        assert!(bee::update::install_binary(&new, &current, Some(SHA)).unwrap());
        let installed = fs::read_to_string(&current).unwrap();
        assert!(installed.contains(SHA));
        bee::update::smoketest(&current, Some(SHA)).unwrap();
    }
}

#[test]
fn checksum_lookup_fails_closed_without_a_matching_entry() {
    assert!(bee::update::checksum_for("bad line\n", "bee.tar.gz").is_err());
    let hash = "a".repeat(64);
    let sums = format!("{}  bee.tar.gz\n", hash);
    assert_eq!(
        bee::update::checksum_for(&sums, "bee.tar.gz").unwrap(),
        hash
    );
    assert!(bee::update::checksum_for(&sums, "other.tar.gz").is_err());
}
