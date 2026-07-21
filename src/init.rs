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
    for dir in ["bee/tasks", "bee/rules", "bee/pipelines", "bee/cache", "bee/deps", "bee/system", "bee/system/hash", "bee/scripts/install"].iter() {
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
    yaml::writer::save_yaml("bee/system/deps.yml", &deps_config)?;
    println!("  Created system/deps.yml");

    file::write_file_content(&String::from("bee/README.md"), &BEE_README.to_string())?;
    println!("  Created bee/README.md");

    file::write_file_content(&String::from("bee/AGENTS.md"), &AGENTS_MD.to_string())?;
    println!("  Created bee/AGENTS.md");

    file::write_file_content(&String::from("bee/CLAUDE.md"), &CLAUDE_MD.to_string())?;
    println!("  Created bee/CLAUDE.md");

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

    let deps_hash = hash::hash_string(&file::get_file_content(&String::from("bee/system/deps.yml"))?);
    file::write_file_content(&String::from("bee/system/hash/deps"), &deps_hash)?;

    println!("Initialization complete. Hash: {}", init_proof.chars().take(8).collect::<String>());
    Ok(())
}

const BEE_README: &str = r#"# Bee - Task Pipeline Orchestration

**Bee** is a task pipeline orchestration tool written in Rust. It lets you define tasks, connect them into pipelines with automatic dependency resolution, and run them in parallel or sequentially.

---

## Table of Contents

- [Building](#building)
- [Project Initialization](#project-initialization)
- [Directory Structure](#directory-structure)
- [Managing Tasks](#managing-tasks)
- [Managing Pipelines](#managing-pipelines)
- [Managing Rules](#managing-rules)
- [Dependency Management](#dependency-management)
- [Dependency Graph Visualization](#dependency-graph-visualization)
- [Backup System](#backup-system)
- [Cache Management](#cache-management)
- [Project Status](#project-status)
- [YAML File Formats](#yaml-file-formats)
- [Trigger System](#trigger-system)
- [Full Configuration Example](#full-configuration-example)

---

## Building

Bee requires Rust (stable). Build with Cargo:

```bash
cargo build          # debug build
cargo build --release  # release build
cargo test           # run tests
```

The binary is at `target/debug/bee` or `target/release/bee`.

---

## Project Initialization

Initialize bee in the current directory:

```bash
bee init
```

This creates the full directory structure and starter files:

```
bee/
├── tasks/
│   ├── build.yml        # starter build task
│   └── test.yml         # starter test task
├── pipelines/
│   └── main.yml         # default pipeline (build -> test)
├── rules/               # (empty)
├── cache/               # trigger states and task run history
├── deps/                # installed dependencies
├── scripts/
│   └── install/         # install scripts for url/git dependencies
├── system/
│   ├── config.yml       # master registry of all tasks/pipelines/rules
│   ├── deps.yml         # external dependency definitions
│   ├── init             # initialization proof (hash)
│   ├── hash/
│   │   ├── init         # SHA-256 of init file
│   │   ├── config       # SHA-256 of config.yml
│   │   └── deps         # SHA-256 of deps.yml
│   ├── scripts/         # install scripts for url/git dependencies
│   └── backup/          # configuration snapshots
├── README.md            # this file
└── .gitignore           # ignores cache/deps/backup/hash
```

---

## Directory Structure

| Directory | Purpose |
|-----------|---------|
| `tasks/` | Task definitions (YAML). Each file defines a single task with a shell command and optional dependencies. |
| `pipelines/` | Pipeline definitions (YAML). Each file lists tasks to run in order, respecting the dependency DAG. |
| `rules/` | Trigger rules (YAML). Defines when tasks should run based on file changes, git state, other tasks, etc. |
| `cache/` | Runtime cache data. Stores trigger states and task execution history. |
| `deps/` | Installed dependencies. Tools and libraries fetched by `bee install`. |
| `system/` | Internal metadata. Config registry, integrity hashes, and backup storage. |
| `system/hash/` | Tamper-detection hashes for config, deps, and init files. |
| `scripts/install/` | Install scripts for url/git dependencies (`bee/scripts/install/<name>.sh`). |
| `system/backup/` | Backup snapshots of tasks, pipelines, and rules. |
| `system/deps.yml` | External dependency definitions — install with `bee install`. |

---

## Key Files

- `system/config.yml` — Master registry of all tasks, pipelines, and rules. Auto-managed by `bee task/pipeline/rule create/delete`. **Do not edit manually.**
- `system/deps.yml` — Dependency definitions (URL, git, or package manager). Hash-protected like config.yml.
- `README.md` — This file.

---

## Managing Tasks

### Create a task

```bash
bee task create <name> [-d <dep>...] [-r <cmd>]
```

| Flag | Description |
|------|-------------|
| `-d <dep>` | Add a dependency (repeatable: `-d build -d test`) |
| `-r <cmd>` | Set the run command (default: `echo "Running <name>..."`) |

```bash
bee task create compile -r "cargo build"
bee task create test -r "cargo test" -d compile
bee task create package -r "make dist" -d test -d lint
```

### Modify a task

```bash
bee task modify <name> [--add-dep <dep>...] [--rm-dep <dep>...] [-r <cmd>]
```

| Flag | Description |
|------|-------------|
| `--add-dep <dep>` | Add a dependency (repeatable) |
| `--rm-dep <dep>` | Remove a dependency (repeatable) |
| `-r <cmd>` | Replace the run command |

```bash
bee task modify test --add-dep lint
bee task modify test --rm-dep lint
bee task modify compile -r "cargo build --release"
```

### List tasks

```bash
bee task list
```

### Run a task

```bash
bee task run <name>
```

Runs the task directly, bypassing pipeline ordering. The trigger system still checks whether the task should execute.

### Delete a task

```bash
bee task delete <name>
```

### Task YAML format

```yaml
name: compile
run: cargo build
depends_on:
  - test
  - lint
```

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Task name (must match filename) |
| `run` | string | Shell command to execute |
| `depends_on` | list[string] | Optional list of task names this task depends on |

---

## Managing Pipelines

### Create a pipeline

```bash
bee pipeline create <name> [-t <task>...]
```

| Flag | Description |
|------|-------------|
| `-t <task>` | Add a task to the pipeline (repeatable: `-t compile -t test`) |

```bash
bee pipeline create ci -t compile -t test -t lint -t deploy
```

### Modify a pipeline

```bash
bee pipeline modify <name> [--add-task <task>...] [--rm-task <task>...]
```

| Flag | Description |
|------|-------------|
| `--add-task <task>` | Add a task (repeatable, skips duplicates) |
| `--rm-task <task>` | Remove a task (repeatable) |

```bash
bee pipeline modify ci --add-task package
bee pipeline modify ci --rm-task deploy
```

### List pipelines

```bash
bee pipeline list
```

### Run a pipeline

```bash
bee pipeline run <name>
```

Runs the full pipeline. The system automatically:
1. Builds a dependency graph (DAG) from task `depends_on` fields
2. Performs topological sort
3. Groups tasks into layers (parallel groups)
4. Runs layers sequentially, tasks within a layer run in parallel (separate threads)
5. On failure: only tasks that depend on the failed task are skipped — independent branches continue
6. Reports exit code, stderr, and color-coded status for each task

```
Pipeline: ci (5 tasks, 2 group(s))
  [1/2] compile, build (parallel)
  - compile: running
  - build: running
build OK
  - compile: FAILED (exit code 1)
  [2/2] lint: SKIPPED (dependency failed)
  [2/2] test: SKIPPED (dependency failed)
  [2/2] deploy
  - deploy: running
deploy OK

Pipeline 'ci' failed: compile
Skipped: lint, test
```

### Run all pipelines

```bash
bee run
```

### Delete a pipeline

```bash
bee pipeline delete <name>
```

### Pipeline YAML format

```yaml
name: ci
tasks:
  - compile
  - test
  - lint
  - package
  - deploy
```

---

## Managing Rules

### Create a rule

```bash
bee rule create <name> --task <task>
```

Creates a `manual` rule (only runs when explicitly requested).

### Modify a rule

```bash
bee rule modify <name> [--task <task>] [--trigger-type <type>] [--trigger-param <key=value>...]
```

| Flag | Description |
|------|-------------|
| `--task <task>` | Change the target task |
| `--trigger-type <type>` | Add a new trigger |
| `--trigger-param k=v` | Parameters for the trigger (repeatable) |

**Trigger params by type:**

| Trigger type | Params |
|-------------|--------|
| `file_change`, `git_changed`, `checksum_changed` | `paths=glob1,glob2` |
| `task_completed`, `task_failed`, `dependency_changed` | `task=name` |
| `env_changed` | `vars=VAR1,VAR2` |
| `dependency_missing` | `path=/path/to/file` |
| `schedule` | `cron=0 * * * *` |
| `git_tag` | `pattern=v1.*` |

```bash
bee rule modify rebuild --trigger-type file_change --trigger-param "paths=src/**/*.rs,src/**/*.c"
bee rule modify rebuild --trigger-type task_completed --trigger-param "task=compile"
bee rule modify rebuild --task test
```

### List rules

```bash
bee rule list
```

### Delete a rule

```bash
bee rule delete <name>
```

### Rule YAML format

```yaml
name: rebuild-on-source
task: compile
triggers:
  - type: manual
  - type: file_change
    paths:
      - src/**/*.c
      - src/**/*.h
```

---

## Dependency Management

Bee can manage external dependencies (tools, libraries).

### List dependencies

```bash
bee install list
```

### Add a dependency

```bash
bee install add <name> --dep-type <type> --source <source> [--version <ver>] [--command <cmd>] [--install-script <script>]
```

Dependency types:
- `package` — system package (requires `--command`, e.g. `sudo apt-get install -y`)
- `url` — download from URL (archive)
- `git` — clone a git repository

For `url` and `git` types you can provide `--install-script` with a shell script that runs after download/clone. The script is saved to `bee/scripts/install/<name>.sh` and executed with working directory set to `bee/deps/<name>/`.

```bash
bee install add mytool url https://example.com/tool.tar.gz --install-script 'make && make install'
bee install add mylib git https://github.com/user/repo --install-script 'cargo build --release'
```

### Modify a dependency

```bash
bee install modify <name> [--dep-type <type>] [--source <src>] [--version <ver>] [--command <cmd>] [--install-script <script>]
```

```bash
bee install modify jq --source jq-full
bee install modify shellcheck --version v0.10.0
bee install modify mytool --install-script 'make clean && make && make install'
```

### Install all dependencies

```bash
bee install run
```

### Remove a dependency

```bash
bee install remove <name>
```

### system/deps.yml format

```yaml
dependencies:
  - name: jq
    type: package
    source: jq
    command: "sudo apt-get install -y"
  - name: shellcheck
    type: url
    source: https://github.com/koalaman/shellcheck/releases/download/v0.9.0/shellcheck-v0.9.0.linux.x86_64.tar.xz
    version: v0.9.0
  - name: mytool
    type: git
    source: https://github.com/user/mytool
    install_script: true
```

When `install_script: true` is set, bee will run `bee/scripts/install/<name>.sh` after download/clone. The script is executed with working directory `bee/deps/<name>/`.

---

## Dependency Graph Visualization

### All pipelines

```bash
bee graph all [format]
```

### Specific pipeline

```bash
bee graph pipeline <name> [format]
```

Formats:
- `tree` (default) — text tree
- `dot` — GraphViz DOT
- `mermaid` — Mermaid diagram

**Example (tree):**
```
Pipeline: ci (5 tasks, 3 groups)
  [1/3] compile (parallel)
  [2/3] test, lint (parallel)
  [3/3] package
  [4/3] deploy
```

**Example (mermaid):**
```
graph LR;
  compile-->test;
  compile-->lint;
  test-->package;
  lint-->package;
  package-->deploy;
```

---

## Backup System

### Create a backup

```bash
bee backup create
```

Copies current tasks, pipelines, rules, and config to `bee/system/backup/<hash>`. Automatically cleans old backups (max 20).

### List backups

```bash
bee backup list
```

### Restore a backup

```bash
bee backup restore <hash>
```

---

## Cache Management

```bash
bee clean
```

Removes the entire `bee/cache/` directory, resetting all trigger states and run history.

---

## Project Status

```bash
bee status
```

Displays:
- Number of pipelines, tasks, and rules
- Pipeline details (task order + cached/pending status)
- Count of cached tasks

---

## YAML File Formats

### config.yml (system/config.yml)

Master registry — automatically updated by `bee task/pipeline/rule create/delete`:

```yaml
tasks:
  - compile
  - test
  - lint
pipelines:
  - ci
  - build-only
rules:
  - compile
  - test
```

> **WARNING:** Do not edit this file manually. Bee manages it automatically through commands.

---

## Trigger System

Rules define **when** tasks should run. Without a rule, a task runs every time.

### Available trigger types

| Trigger | Description | Parameters |
|---------|-------------|------------|
| `manual` | Only on explicit request | — |
| `always` | Always run | — |
| `file_change` | Run when matching files change | `paths` — list of globs |
| `git_changed` | Run when git-tracked files change | `paths` — list of globs |
| `task_completed` | Run when another task succeeds | `task` — task name |
| `task_failed` | Run when another task fails | `task` — task name |
| `dependency_changed` | Run when upstream task changes | `task` — task name |
| `env_changed` | Run when environment variables change | `vars` — list of var names |
| `dependency_missing` | Run when a file/binary is missing | `path` — file path |
| `checksum_changed` | Run when file checksums change | `paths` — list of globs |
| `schedule` | Run on a cron schedule | `cron` — cron expression |
| `git_tag` | Run on git tags | `pattern` — optional pattern |

### Examples

```yaml
# Auto-compile when sources change
name: auto-compile
task: compile
triggers:
  - type: file_change
    paths:
      - src/**/*.rs

# Run test after compile succeeds
name: test-after-compile
task: test
triggers:
  - type: task_completed
    task: compile

# Deploy only manually
name: manual-deploy
task: deploy
triggers:
  - type: manual

# Retry deploy when package fails
name: retry-deploy
task: deploy
triggers:
  - type: task_failed
    task: package
  - type: manual

# Build when binary is missing
name: build-if-missing
task: compile
triggers:
  - type: dependency_missing
    path: target/release/myapp
```

### How triggers work

1. When `bee task run <name>` is called, the system checks all rules assigned to that task
2. If no rules exist → task always runs
3. If rules exist → **any** trigger returning `true` causes the task to run
4. The `manual` trigger never returns `true` automatically
5. Trigger states are cached in `bee/cache/triggers/`
6. `bee clean` resets all trigger states

---

## Full Configuration Example

### 1. Initialize

```bash
bee init
```

### 2. Create tasks (all from CLI)

```bash
bee task create compile -r "cargo build --release"
bee task create test -r "cargo test" -d compile
bee task create lint -r "cargo clippy -- -D warnings" -d compile
bee task create package -r "tar -czf release.tar.gz target/release/myapp" -d test -d lint
bee task create deploy -r "scp release.tar.gz user@server:/opt/app/" -d package
```

### 3. Modify a task

```bash
bee task modify compile -r "cargo build --release --features full"
bee task modify test --add-dep compile
```

### 4. Create a pipeline with tasks

```bash
bee pipeline create ci -t compile -t test -t lint -t package -t deploy
```

### 5. Modify the pipeline

```bash
bee pipeline modify ci --add-task package
bee pipeline modify ci --rm-task deploy
```

### 6. Create and modify rules

```bash
bee rule create rebuild --task compile
bee rule modify rebuild --trigger-type file_change --trigger-param "paths=src/**/*.rs"
bee rule modify rebuild --trigger-type task_completed --trigger-param "task=compile"
```

### 7. Run

```bash
bee pipeline run ci    # Run the full pipeline
bee task run deploy    # Run a single task
bee graph pipeline ci  # Show dependency graph
bee status             # Show project status
```

---

## Integrity Security

Bee stores SHA-256 hashes:
- `bee/system/init` — initialization proof
- `bee/system/hash/init` — hash of init file
- `bee/system/hash/config` — hash of config.yml
- `bee/system/hash/deps` — hash of deps.yml

The system verifies integrity on every operation. If hashes don't match, bee returns "run bee init first".
"#;

const AGENTS_MD: &str = r#"# AGENTS.md - Instructions for AI Agents

## IMPORTANT: Always read `bee/README.md` before working with the bee system!

Before any operation on the bee system (modifying tasks, pipelines, rules, running commands) you **MUST** read the `bee/README.md` file. It contains:

- Full documentation of all commands
- Directory structure
- YAML file formats (tasks, pipelines, rules, system/deps.yml)
- Trigger system and run rules
- Integrity security information (SHA-256 hashes)

## Rules for working with bee

1. **Never edit `bee/system/config.yml` manually** — bee manages it automatically via `bee task/pipeline/rule create/delete`
2. **Never delete `bee/system/hash/` directories** — they contain integrity hashes
3. **Before running a bee command** make sure you are in the parent directory of `bee/` (where `bee/system/init` exists)
4. **After modifying tasks** consider running `bee clean` to reset trigger states
5. **Before creating new tasks/pipelines/rules** check existing ones with `bee list`
7. **Never add new rule/pipeline/task without running command** — this elements won't be visible for bee system

## File Structure

```
bee/
├── tasks/<name>.yml      # Task: name, run, depends_on
├── pipelines/<name>.yml   # Pipeline: name, tasks
├── rules/<name>.yml       # Rule: name, task, triggers
├── deps.yml               # Dependencies: dependencies[]
├── system/config.yml      # Registry (auto-managed)
├── system/deps.yml        # Dependencies (hash-protected)
└── README.md              # DOCUMENTATION - always read first!
```

## Available Commands

```bash
bee init                                         # Initialize project
bee list                                         # List everything
bee status                                       # Project status
bee run                                          # Run all pipelines
bee clean                                        # Clear cache

bee task create <name> [-d <dep>...] [-r <cmd>]  # Create task
bee task modify <name> [--add-dep/--rm-dep/-r]   # Modify task
bee task list                                    # List tasks
bee task run <name>                              # Run task
bee task delete <name>                           # Delete task

bee pipeline create <name> [-t <task>...]        # Create pipeline
bee pipeline modify <name> [--add/--rm-task]     # Modify pipeline
bee pipeline list                                # List pipelines
bee pipeline run <name>                          # Run pipeline
bee pipeline delete <name>                       # Delete pipeline

bee rule create <name> --task <t>                # Create rule
bee rule modify <name> [--task/--trigger-*]      # Modify rule
bee rule list                                    # List rules
bee rule delete <name>                           # Delete rule

bee graph all [tree|dot|mermaid]                 # Graph all pipelines
bee graph pipeline <name> [tree|dot|mermaid]     # Graph specific pipeline

bee install list                                 # List dependencies
bee install run                                  # Install dependencies
bee install add ...                              # Add dependency
bee install modify <name> [--dep-type/--source]  # Modify dependency
bee install remove <name>                        # Remove dependency

bee backup create                                # Create backup
bee backup list                                  # List backups
bee backup restore <hash>                        # Restore backup
```
"#;

const CLAUDE_MD: &str = r#"# CLAUDE.md

> **This file redirects to the single source of truth.**

See **`AGENTS.md`** for all instructions, rules, and command reference.

Also read **`bee/README.md`** for full documentation before any bee operation.
"#;
