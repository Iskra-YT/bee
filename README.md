# Bee - Task Pipeline Orchestration

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
├── system/
│   ├── config.yml       # master registry of all tasks/pipelines/rules
│   ├── init             # initialization proof (hash)
│   ├── hash/
│   │   ├── init         # SHA-256 of init file
│   │   └── config       # SHA-256 of config.yml
│   └── backup/          # configuration snapshots
├── deps.yml             # external dependency definitions
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
| `system/hash/` | Tamper-detection hashes for config and init files. |
| `system/backup/` | Backup snapshots of tasks, pipelines, and rules. |
| `deps.yml` | Project dependencies — define external tools to install with `bee install`. |

---

## Key Files

- `system/config.yml` — Master registry of all tasks, pipelines, and rules. Auto-managed by `bee task/pipeline/rule create/delete`. **Do not edit manually.**
- `deps.yml` — Dependency definitions (URL, git, or package manager).
- `README.md` — This file.

---

## Managing Tasks

### Create a task

```bash
bee task create <name>
```

Creates `bee/tasks/<name>.yml` with a default echo command.

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
run: gcc -o main main.c && echo "Compiled!"
depends_on:
  - setup
```

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Task name (must match filename) |
| `run` | string | Shell command to execute |
| `depends_on` | list[string] | Optional list of task names this task depends on |

Tasks without `depends_on` or with an empty list have no dependencies and run first.

---

## Managing Pipelines

### Create a pipeline

```bash
bee pipeline create <name>
```

Creates an empty `bee/pipelines/<name>.yml`.

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

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Pipeline name |
| `tasks` | list[string] | List of task names (order doesn't matter — DAG sorts them) |

---

## Managing Rules

### Create a rule

```bash
bee rule create <name> --task <task>
```

Creates a `manual` rule (only runs when explicitly requested).

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
bee install add <name> --dep-type <type> --source <source> [--version <ver>] [--command <cmd>]
```

Dependency types:
- `package` — system package (requires `--command`, e.g. `sudo apt-get install -y`)
- `url` — download from URL (archive)
- `git` — clone a git repository

### Install all dependencies

```bash
bee install run
```

### Remove a dependency

```bash
bee install remove <name>
```

### deps.yml format

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
```

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

### 2. Create tasks

```bash
bee task create compile
bee task create test
bee task create lint
bee task create package
bee task create deploy
```

### 3. Edit task files

`bee/tasks/compile.yml`:
```yaml
name: compile
run: cargo build --release
depends_on: []
```

`bee/tasks/test.yml`:
```yaml
name: test
run: cargo test
depends_on:
  - compile
```

`bee/tasks/lint.yml`:
```yaml
name: lint
run: cargo clippy -- -D warnings
depends_on:
  - compile
```

`bee/tasks/package.yml`:
```yaml
name: package
run: tar -czf release.tar.gz target/release/myapp
depends_on:
  - test
  - lint
```

`bee/tasks/deploy.yml`:
```yaml
name: deploy
run: scp release.tar.gz user@server:/opt/app/
depends_on:
  - package
```

### 4. Create a pipeline

```bash
bee pipeline create ci
```

`bee/pipelines/ci.yml`:
```yaml
name: ci
tasks:
  - compile
  - test
  - lint
  - package
  - deploy
```

### 5. Create rules

`bee/rules/auto-compile.yml`:
```yaml
name: auto-compile
task: compile
triggers:
  - type: file_change
    paths:
      - src/**/*.rs
```

### 6. Run

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

The system verifies integrity on every operation. If hashes don't match, bee returns "run bee init first".
