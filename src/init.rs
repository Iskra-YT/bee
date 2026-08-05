use std::fs;
use anyhow::Result;
use crate::parser;
use crate::yaml;
use crate::hash;
use crate::time;
use crate::file;
use std::io::{self, Write};
use std::path::Path;

pub fn run_init(name: Option<&str>, yes: bool) -> Result<()> {
    if file::check_bee_directory() {
        println!("bee is already initialized in this directory");
        return Ok(());
    }

    if let Some(parent) = file::find_parent_system() {
        let parent_abs = fs::canonicalize(&parent).unwrap_or_else(|_| parent.clone());
        println!("Detected a parent bee system at '{}'.", parent_abs.display());
        print_parent_chain(&parent)?;

        if yes {
            return run_init_subsystem(&parent, name);
        }

        print!("Create a subsystem of it? [S]ubsystem / [F]ull / [C]ancel [S]: ");
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        match answer.trim().to_lowercase().chars().next() {
            Some('f') => println!("Creating an independent bee project instead."),
            Some('c') => {
                println!("Cancelled.");
                return Ok(());
            }
            _ => return run_init_subsystem(&parent, name),
        }
    }

    run_init_full()
}

fn print_parent_chain(parent: &Path) -> Result<()> {
    let mut current = parent.to_path_buf();
    let mut depth = 0usize;
    loop {
        let abs = fs::canonicalize(&current).unwrap_or_else(|_| current.clone());
        let label = if depth == 0 { "parent" } else { "ancestor" };
        let suffix = if current.join("bee/system/parent").exists() {
            " (this system is itself a subsystem)"
        } else {
            ""
        };
        println!("  {} bee system: {}{}", label, abs.display(), suffix);
        let link = current.join("bee/system/parent");
        let Ok(content) = fs::read_to_string(&link) else { break };
        let up = Path::new(content.trim()).to_path_buf();
        current = if up.is_absolute() { up } else { current.join(up) };
        depth += 1;
        if depth >= 32 {
            break;
        }
    }
    Ok(())
}

fn run_init_full() -> Result<()> {
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
        pipelines: vec![],
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

    file::write_file_content(&String::from("bee/.gitignore"), &BEE_GITIGNORE.to_string())?;
    println!("  Created bee/.gitignore");

    write_init_proof()?;

    println!("Initialization complete. Hash: {}", hash::hash_string(&file::get_file_content(&String::from("bee/system/init"))?).chars().take(8).collect::<String>());
    Ok(())
}

fn run_init_subsystem(parent: &Path, name: Option<&str>) -> Result<()> {
    let _ = fs::remove_dir_all("bee");
    for dir in ["bee/tasks", "bee/rules", "bee/pipelines", "bee/cache", "bee/system", "bee/system/hash"].iter() {
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
        pipelines: vec![],
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

    let parent_rel = parent.to_string_lossy().to_string();
    file::write_file_content(&String::from("bee/system/parent"), &parent_rel)?;
    println!("  Created system/parent -> {}", parent_rel);

    file::write_file_content(&String::from("bee/.gitignore"), &BEE_GITIGNORE.to_string())?;
    println!("  Created bee/.gitignore");

    let init_proof = write_init_proof()?;

    let subsystem_name = name.map(String::from).unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|d| d.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| String::from("subsystem"))
    });

    register_subsystem(parent, &subsystem_name, &init_proof)?;

    println!("Subsystem '{}' created and registered in parent.", subsystem_name);
    Ok(())
}

fn write_init_proof() -> Result<String> {
    let init_proof: String = hash::hash_string(time::get_timestamp_string().as_str());
    file::write_file_content(&String::from("bee/system/init"), &init_proof)?;

    let init_hash = hash::hash_string(&file::get_file_content(&String::from("bee/system/init"))?);
    hash::write_hash_file("bee/system/hash/init", &init_hash)?;

    let config_hash = hash::hash_string(&file::get_file_content(&String::from("bee/system/config.yml"))?);
    hash::write_hash_file("bee/system/hash/config", &config_hash)?;

    Ok(init_proof)
}

fn register_subsystem(parent: &Path, name: &str, init_proof: &str) -> Result<()> {
    let parent_abs = fs::canonicalize(parent)?;
    let cwd = std::env::current_dir()?;

    let subsystems_path = parent_abs.join("bee/system/subsystems.yml");
    let mut config: parser::SubsystemsConfig = if subsystems_path.exists() {
        let content = file::get_file_content(&subsystems_path.to_string_lossy().to_string())?;
        serde_yaml::from_str(&content).unwrap_or_default()
    } else {
        parser::SubsystemsConfig::default()
    };

    let rel_path = file::relative_path(&parent_abs, &cwd)
        .to_string_lossy()
        .to_string();

    let subsystem_hash = hash::hash_string(&format!("{}|{}|{}", name, rel_path, init_proof));

    if let Some(existing) = config.subsystems.iter().find(|s| s.path == rel_path) {
        let existing_name = existing.name.clone();
        config.subsystems.retain(|s| s.path != rel_path);
        println!("Re-registering subsystem '{}' at path '{}'.", existing_name, rel_path);
    } else if let Some(existing) = config.subsystems.iter().find(|s| s.name == name) {
        let existing_path = existing.path.clone();
        anyhow::bail!(
            "Subsystem name '{}' is already registered at path '{}'. Use --name to choose a different name.",
            name,
            existing_path
        );
    }

    config.subsystems.push(parser::Subsystem {
        name: String::from(name),
        path: rel_path,
        date: time::get_timestamp_string(),
        hash: subsystem_hash,
    });

    yaml::writer::save_yaml(
        subsystems_path.to_str().unwrap_or("bee/system/subsystems.yml"),
        &config,
    )?;

    let subsystems_hash = hash::hash_string(&file::get_file_content(&subsystems_path.to_string_lossy().to_string())?);
    hash::write_hash_file(
        &parent_abs.join("bee/system/hash/subsystems").to_string_lossy().to_string(),
        &subsystems_hash,
    )?;

    Ok(())
}

const BEE_GITIGNORE: &str = r#"cache/
 deps/
 system/backup/
 "#;

pub const BEE_README: &str = r#"# Bee - Task Pipeline Orchestration

**Bee** is a task pipeline orchestration tool written in Rust. It lets you define tasks, connect them into pipelines with automatic dependency resolution, and run them in parallel or sequentially.

---

## Table of Contents

- [Building](#building)
- [Updating](#updating)
- [Project Initialization](#project-initialization)
- [Subsystems](#subsystems)
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

Show the installed bee version (the git commit hash recorded at build/install time):

```bash
bee version
```

---

## Updating

Update bee to the latest version from GitHub:

```bash
sudo bee update
```

This will:
1. Clone the latest bee source from GitHub to a temporary directory
2. Build bee in release mode
3. Replace the currently running binary
4. Clean up temporary files

> **Note:** Requires `sudo` because the binary is typically installed to `/usr/local/bin/`.

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

## Subsystems

Bee detects when you run `bee init` inside an already-initialized bee project and proposes creating a **subsystem** — a lightweight, self-contained bee instance scoped to the current folder.

### Creating a subsystem

```bash
bee init
```

Bee walks up the directory tree, finds the nearest valid parent bee system, and asks:

```
Detected a parent bee system at '/path/to/parent'.
Create a subsystem of it in this directory? [Y/n]
```

- `Y` (default) — creates a lightweight subsystem:
  - own `bee/tasks/`, `bee/pipelines/`, `bee/rules/` — works independently
  - minimal `bee/system/` (`init`, `config.yml`, hashes)
  - `bee/system/parent` — relative pointer back to the parent system
  - registers itself in the parent's `bee/system/subsystems.yml`
- `n` — creates a normal, fully independent bee project.

Flags:

```bash
bee init --yes          # auto-accept subsystem creation (no prompt)
bee init --name auth    # override subsystem name (default: current directory name)
```

### Subsystem commands

Within a subsystem the basic commands operate on the subsystem's own resources:
`bee task`, `bee pipeline`, `bee rule`, `bee run`, `bee status`, `bee subsystem`.
`bee install`, `bee backup`, and `bee graph` are **not available in a subsystem**.

```bash
bee subsystem list    # in a subsystem: shows the parent + sibling subsystems (marks current)
                      # in a parent: lists registered subsystems
bee subsystem status  # shows the link to the parent system and registration details
```

### Task connections (parent ↔ subsystem)

A parent task can invoke a task or pipeline inside a subsystem, and tasks on either side can react to each other through trigger rules.

**Parent calls a child task or pipeline:**

```bash
bee subsystem run <subsystem> task <task>
bee subsystem run <subsystem> pipeline <pipeline>
```

Run it from a parent task's `run` command to chain systems, e.g.:

```yaml
name: build-all
run: bee subsystem run auth task build && bee subsystem run payments pipeline build
```

**Parent reacts to a subsystem** — create a rule in the parent with one of:

```bash
bee rule modify on-child --trigger-type subtask_completed --trigger-param subsystem=auth --trigger-param task=build
bee rule modify on-child --trigger-type subtask_failed    --trigger-param subsystem=auth --trigger-param task=build
bee rule modify on-child --trigger-type subtask_changed   --trigger-param subsystem=auth --trigger-param task=build
bee rule modify on-child --trigger-type subsystem_changed --trigger-param subsystem=auth
```

**Subsystem reacts to the parent** — create a rule inside the subsystem:

```bash
bee rule modify after-parent --trigger-type parent_task_completed --trigger-param task=deploy
bee rule modify after-parent --trigger-type parent_task_failed    --trigger-param task=deploy
```

The state of each task lives in the owning system's `bee/cache/triggers/`, so `subtask_*`/`subsystem_changed` read the subsystem's cache from the parent, and `parent_task_*` reads the parent's cache from the subsystem.

### Parent registry

The parent system stores `bee/system/subsystems.yml` (name, path, date, hash) and a tamper-detection hash in `bee/system/hash/subsystems`. The file is created on demand, so existing projects work without changes.

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
bee pipeline create <name> [-t <task>...] [-p <pipeline>...]
```

| Flag | Description |
|------|-------------|
| `-t <task>` | Add a task to the pipeline (repeatable: `-t compile -t test`) |
| `-p <pipeline>` | Add a subpipeline to invoke (repeatable: `-p build -p deploy`) |

```bash
bee pipeline create ci -t compile -t test -t lint -t deploy
bee pipeline create full -p build -p test -p deploy
```

### Modify a pipeline

```bash
bee pipeline modify <name> [--add-task <task>...] [--rm-task <task>...] [--add-pipeline <pipeline>...] [--rm-pipeline <pipeline>...]
```

| Flag | Description |
|------|-------------|
| `--add-task <task>` | Add a task (repeatable, skips duplicates) |
| `--rm-task <task>` | Remove a task (repeatable) |
| `--add-pipeline <pipeline>` | Add a subpipeline (repeatable, skips duplicates) |
| `--rm-pipeline <pipeline>` | Remove a subpipeline (repeatable) |

```bash
bee pipeline modify ci --add-task package
bee pipeline modify ci --rm-task deploy
bee pipeline modify full --add-pipeline lint --rm-pipeline test
```

### Subpipelines

A pipeline can invoke other pipelines. When you run it, all subpipelines run first
(in the order listed), then the pipeline's own tasks run through the normal DAG:

```yaml
name: full
tasks:
  - package
  - deploy
pipelines:
  - build
  - test
  - lint
```

```bash
bee pipeline run full
```

Subpipelines run recursively (a subpipeline can itself have subpipelines). Cycles are
detected automatically and reported, e.g. `build -> ci -> build`.

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
pipelines:
  - build        # optional subpipelines, run before this pipeline's tasks
```

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Pipeline name |
| `tasks` | list[string] | List of task names (order doesn't matter — DAG sorts them) |
| `pipelines` | list[string] | Optional list of subpipelines invoked before this pipeline's tasks |

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
| `subtask_completed`, `subtask_failed`, `subtask_changed` | `subsystem=name`, `task=name` |
| `parent_task_completed`, `parent_task_failed` | `task=name` |
| `subsystem_changed` | `subsystem=name` |

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
bee install add <name> --dep-type <type> --source <source> [--version <ver>] [--command <cmd>] [--install-script <script>] [--path <path>]
```

Dependency types:
- `package` — system package (requires `--command`, e.g. `sudo apt-get install -y`)
- `url` — download from URL (archive)
- `git` — clone a git repository
- `command` — run an arbitrary shell command (requires `--command`)
- `manager` — run a package manager in a specific directory (requires `--path`, optional `--command`, default: `npm install`)

For `url` and `git` types you can provide `--install-script` with a shell script that runs after download/clone. The script is saved to `bee/scripts/install/<name>.sh` and executed with working directory set to `bee/deps/<name>/`.

```bash
bee install add mytool url https://example.com/tool.tar.gz --install-script 'make && make install'
bee install add mylib git https://github.com/user/repo --install-script 'cargo build --release'
bee install add npm-packages --dep-type manager --path src/frontend --command "npm install"
bee install add pip-deps --dep-type manager --path backend --command "pip install -r requirements.txt"
```

### Modify a dependency

```bash
bee install modify <name> [--dep-type <type>] [--source <src>] [--version <ver>] [--command <cmd>] [--install-script <script>] [--path <path>]
```

```bash
bee install modify jq --source jq-full
bee install modify shellcheck --version v0.10.0
bee install modify mytool --install-script 'make clean && make && make install'
bee install modify npm-packages --path frontend
bee install modify pip-deps --command "pip install -r requirements.txt"
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
  - name: npm-packages
    type: manager
    path: src/frontend
    command: npm install
  - name: pip-deps
    type: manager
    path: backend
    command: pip install -r requirements.txt
```

When `install_script: true` is set, bee will run `bee/scripts/install/<name>.sh` after download/clone. The script is executed with working directory `bee/deps/<name>/`.

The `manager` type runs a package manager command inside the specified `path` directory. Use it for project-local dependencies (npm, pip, cargo, etc.).

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
| `subtask_completed` | Run when a task in a subsystem succeeds | `subsystem`, `task` |
| `subtask_failed` | Run when a task in a subsystem fails | `subsystem`, `task` |
| `subtask_changed` | Run when a task in a subsystem changes state | `subsystem`, `task` |
| `parent_task_completed` | Run when a task in the parent system succeeds (in a subsystem) | `task` |
| `parent_task_failed` | Run when a task in the parent system fails (in a subsystem) | `task` |
| `subsystem_changed` | Run when any task state in a subsystem changes | `subsystem` |

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
- `bee/system/hash/subsystems` — hash of the parent's subsystems registry

Each `bee/system/hash/<file>` entry has a companion `bee/system/hash/<file>.bee` containing the bee git hash (`bee version`) that created it.

The system verifies integrity on every operation. If hashes don't match, bee returns "run bee init first".
"#;

pub const AGENTS_MD: &str = r#"# AGENTS.md - Instructions for AI Agents

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
8. **Subsystems**: running `bee init` in a subfolder of an existing bee project proposes creating a subsystem (own tasks/pipelines/rules linked to the parent). Inspect relationships with `bee subsystem list` / `bee subsystem status`. In a subsystem `bee install`, `bee backup`, and `bee graph` are unavailable.

## File Structure

```
bee/
├── tasks/<name>.yml      # Task: name, run, depends_on
├── pipelines/<name>.yml   # Pipeline: name, tasks, optional pipelines (subpipelines)
├── rules/<name>.yml       # Rule: name, task, triggers
├── deps.yml               # Dependencies: dependencies[]
├── system/config.yml      # Registry (auto-managed)
├── system/deps.yml        # Dependencies (hash-protected)
└── README.md              # DOCUMENTATION - always read first!
```

## Available Commands

```bash
bee init                                         # Initialize project
bee init --name <n> --yes                         # Create subsystem without prompt
bee update                                        # Update bee to latest version
bee version                                       # Show installed bee git hash
bee list                                         # List everything
bee status                                       # Project status
bee run                                          # Run all pipelines
bee clean                                        # Clear cache

bee task create <name> [-d <dep>...] [-r <cmd>]  # Create task
bee task modify <name> [--add-dep/--rm-dep/-r]   # Modify task
bee task list                                    # List tasks
bee task run <name>                              # Run task
bee task delete <name>                           # Delete task

bee pipeline create <name> [-t <task>...] [-p <pipeline>...] # Create pipeline
bee pipeline modify <name> [--add/--rm-task] [--add/--rm-pipeline] # Modify pipeline
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

bee subsystem list                                # List subsystems (parent or siblings)
bee subsystem status                              # Show subsystem link to parent
bee subsystem run <sub> task <task>               # Run a task inside a subsystem
bee subsystem run <sub> pipeline <pipeline>       # Run a pipeline inside a subsystem

bee docs update                                  # Regenerate bee/*.md docs from built-in templates

# Subsystem-aware trigger types:
#   subtask_completed/failed/changed: --trigger-param subsystem=<sub> --trigger-param task=<task>
#   parent_task_completed/failed:     --trigger-param task=<task>
#   subsystem_changed:                --trigger-param subsystem=<sub>
```
"#;

pub const CLAUDE_MD: &str = r#"# CLAUDE.md

> **This file redirects to the single source of truth.**

See **`AGENTS.md`** for all instructions, rules, and command reference.

Also read **`bee/README.md`** for full documentation before any bee operation.
"#;
