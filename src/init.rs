use std::fs;
use anyhow::Result;
use crate::parser;
use crate::yaml;
use crate::hash;
use crate::time;
use crate::file;

pub fn run_init() -> Result<()> {
    if file::check_bee_directory() {
        println!("bee is already initialized in this directory");
        return Ok(());
    }
    let _ = fs::remove_dir_all("bee");
    for dir in ["bee/tasks", "bee/rules", "bee/pipelines", "bee/cache", "bee/deps", "bee/system", "bee/system/hash"].iter() {
        fs::create_dir_all(dir)?;
    }

    let task_build_config = parser::Task {
        name: String::from("build"),
        run: String::from("echo \"Building...!\""),
        depends_on: Some(vec![]),
    };

    yaml::writer::save_yaml("bee/tasks/build.yml", &task_build_config)?;
    println!("  Created tasks/build.yml");

    let task_test_config = parser::Task {
        name: String::from("test"),
        run: String::from("echo \"Testing...!\""),
        depends_on: Some(vec![String::from("build")]),
    };

    yaml::writer::save_yaml("bee/tasks/test.yml", &task_test_config)?;
    println!("  Created tasks/test.yml");

    let pipeline_main_config = parser::Pipeline {
        name: String::from("main"),
        tasks: vec![String::from("build"), String::from("test")],
    };

    yaml::writer::save_yaml("bee/pipelines/main.yml", &pipeline_main_config)?;
    println!("  Created pipelines/main.yml");

    let main_config = parser::MainConfig {
        tasks: vec![String::from("build"), String::from("test")],
        rules: vec![],
        pipelines: vec![String::from("main")]
    };

    yaml::writer::save_yaml("bee/system/config.yml", &main_config)?;
    println!("  Created system/config.yml");

    let deps_config = parser::DepsConfig {
        dependencies: vec![],
    };
    yaml::writer::save_yaml("bee/deps.yml", &deps_config)?;
    println!("  Created deps.yml");

    let bee_readme = r#"# Bee Project

This directory is managed by **bee** — a task pipeline orchestration tool.

## Directory Structure

| Directory | Purpose |
|-----------|---------|
| `tasks/` | Task definitions (YAML). Each file defines a single task with a shell command and optional dependencies. |
| `pipelines/` | Pipeline definitions (YAML). Each file lists tasks to run in order, respecting dependency graph. |
| `rules/` | Trigger rules (YAML). Defines when tasks should run based on file changes, git state, other tasks, etc. |
| `cache/` | Runtime cache data. Stores trigger states and task execution history. |
| `deps/` | Installed dependencies. Tools and libraries fetched by `bee install`. |
| `system/` | Internal metadata. Config registry, integrity hashes, and backup storage. |
| `system/hash/` | Tamper-detection hashes for config and init files. |
| `system/backup/` | Backup snapshots of tasks, pipelines, and rules. |
| `deps.yml` | Project dependencies — define external tools to install with `bee install`. |

## Key Files

- `system/config.yml` — Master registry of all tasks, pipelines, and rules.
- `deps.yml` — Dependency definitions (URL, git, or package manager).
- `README.md` — This file.

## Rules System

Rules define **when** tasks should run. Each rule specifies a task and a list of triggers:

- `manual` — Only run when explicitly requested (default)
- `always` — Always run
- `file_change` — Run when files matching patterns change
- `git_changed` — Run when git-tracked files change
- `task_completed` — Run when another task completes
- `task_failed` — Run when another task fails
- `dependency_changed` — Run when upstream task changes
- `env_changed` — Run when environment variables change
- `dependency_missing` — Run when a file/binary is missing
- `checksum_changed` — Run when file checksums change
- `schedule` — Run on a cron schedule
- `git_tag` — Run on git tags
"#;

    file::write_file_content(&String::from("bee/README.md"), &bee_readme.to_string())?;
    println!("  Created bee/README.md");

    let bee_gitignore = r#"cache/
deps/
system/backup/
"#;

    file::write_file_content(&String::from("bee/.gitignore"), &bee_gitignore.to_string())?;
    println!("  Created bee/.gitignore");

    let init_proof: String = hash::hash_string(time::get_timestamp_string().as_str());
    file::write_file_content(&String::from("bee/system/init"), &init_proof)?;

    let init_hash = hash::hash_string(&file::get_file_content(&String::from("bee/system/init"))?);
    file::write_file_content(&String::from("bee/system/hash/init"), &init_hash)?;

    let config_hash = hash::hash_string(&file::get_file_content(&String::from("bee/system/config.yml"))?);
    file::write_file_content(&String::from("bee/system/hash/config"), &config_hash)?;

    println!("Initialization complete. Hash: {}", init_proof.chars().take(8).collect::<String>());
    Ok(())
}