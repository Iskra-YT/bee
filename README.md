# Bee - Task Pipeline Orchestration

**Bee** is a task pipeline orchestration tool written in Rust. It lets you define tasks, connect them into pipelines with automatic dependency resolution, and run them in parallel or sequentially.

---

## Table of Contents

- [Building](#building)
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
- [Task Logs](#task-logs)
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

The git commit hash of the source is embedded at build time. Show the installed version:

```bash
bee version
```

This prints the `git rev-parse HEAD` hash recorded when the binary was built/installed (e.g. by `bee update`).

### Update project documentation

`bee init` also writes `bee/README.md`, `bee/AGENTS.md`, and `bee/CLAUDE.md` inside each project. Regenerate them from the built-in templates with:

```bash
bee docs update
```

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
├── logs/                # per-task execution logs (<task>.log)
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
| `logs/` | Task execution logs. Each task writes its stdout/stderr to `bee/logs/<task>.log` instead of the console. |
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

Bee supports three execution modes plus legacy compatibility.

#### Process executor (recommended for ordinary commands)

```yaml
name: build
executor: process
program: cargo
args:
  - build
  - --release
working_dir: .
```

#### Shell executor (for shell syntax like `&&`, pipes, redirects)

```yaml
name: complex-build
executor: shell
shell: bash
run: cargo build && cargo test
```

#### Script executor (interpreter, with fallback)

```yaml
name: generate-report
executor: script
interpreter: python
script: scripts/report.py
args:
  - --format
  - json
```

`interpreter` accepts either one string or a YAML list of alternatives. Bee checks candidates in order and uses the first one available as a path or on `PATH`:

```yaml
name: build-backend
executor: script
interpreter:
  - python3
  - python
  - py
script: scripts/maintenance/build.py
args:
  - backend
```

The same syntax works as a platform override, e.g. `windows: {interpreter: [python, py]}`.

#### Legacy format (backward compatible)

```yaml
name: compile
run: gcc -o main main.c && echo "Compiled!"
depends_on:
  - setup
```

Tasks without `executor` and with `run` keep the previous shell-based execution semantics for compatibility.

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Task name (must match filename) |
| `depends_on` | list[string] | Optional task dependencies |
| `executor` | string | Built-in mode (`process`, `shell`, `script`, `legacy`) |
| `run` | string | Command string for `shell`/legacy tasks |
| `program` | string | Executable for `process` executor |
| `args` | list[string] | Arguments for `process`/`script` executors |
| `working_dir` | string | Working directory (relative to project root or absolute) |
| `env` | map[string,string] | Environment variables added for the task |
| `shell` | string | Shell for `shell` executor: `bash`, `sh`, `powershell`, `cmd` |
| `interpreter` | string or list[string] | Interpreter for `script` executor; the first available candidate wins |
| `script` | string | Script file for `script` executor |
| `platforms` | map | Optional per-platform overrides: `linux`, `windows`, `macos` |

#### Platform-specific overrides

```yaml
name: start-web
executor: process
program: npm
args:
  - run
  - dev
working_dir: web
platforms:
  linux:
    program: npm
  windows:
    program: npm.cmd
  macos:
    program: npm
```

Platform overrides are merged over the default task configuration. Supported platform keys are `linux`, `windows`, and `macos`.

#### Windows notes

- Native executables are launched directly with `std::process::Command`.
- `.cmd` and `.bat` files are executed through `cmd.exe /C` on Windows.
- Do not assume `.sh` files are directly executable on Windows; use `executor: shell` or an explicit interpreter.
- Shell execution is platform-aware; do not assume `bash` is available on Windows.
- With an interpreter list, `python` matches `python.exe` via `PATHEXT` (default `.EXE;.CMD;.BAT;.COM`).

#### Cross-platform behavior

- `process` executor does not use a shell; arguments are passed as a structured list.
- `shell` executor is opt-in for shell syntax.
- `script` executor runs an interpreter (or the first available interpreter candidate) with a script path.
- Legacy `run` tasks remain supported.
- Interpreter fallback examples:
  - Linux/macOS: `interpreter: [python3, python]`
  - Windows: `interpreter: [python, py]`
- Candidates are checked sequentially with the first interpreter found in `PATH`; paths containing a separator are checked directly.
- An empty interpreter list or a blank candidate is rejected during validation.
- If no candidate is available, Bee reports all attempted interpreters, e.g. `no available interpreter found; tried: python3, python`.
- Fallback is discovery-only: once an interpreter has been launched and fails, Bee does not try the next candidate.

---

## Managing Pipelines

### Create a pipeline

```bash
bee pipeline create <name> [-t <task>...] [-p <pipeline>...]
```

| Flag | Description |
|------|-------------|
| `-t <task>` | Add a task to the pipeline (repeatable) |
| `-p <pipeline>` | Add a subpipeline to invoke (repeatable) |

### List pipelines

```bash
bee pipeline list
```

### Run a pipeline

```bash
bee pipeline run <name>
```

Runs the full pipeline. The system automatically:
1. Runs all subpipelines first (in order), recursively with cycle detection
2. Builds a dependency graph (DAG) from task `depends_on` fields
3. Performs topological sort
4. Groups tasks into layers (parallel groups)
5. Runs layers sequentially, tasks within a layer run in parallel (separate threads)

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
```

```bash
bee pipeline create build -t compile
bee pipeline create full -t package -p build
bee pipeline run full
```

Subpipelines run recursively (a subpipeline can itself have subpipelines). Cycles are
detected automatically, e.g. `build -> ci -> build`.

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

## Task Logs

When a task runs, its output is **not** printed to the console (task outputs would interleave across parallel threads). Instead, every task writes its stdout/stderr to a log file:

```
bee/logs/<taskname>.log
```

The log file is overwritten on each run. On failure the log contains the exit code, stderr, and any stdout produced. The console only shows the task status lines (`running`, `FAILED`, `SKIPPED`, etc.).

```bash
cat bee/logs/compile.log   # inspect what the compile task output
```

`bee clean` does not remove `bee/logs/` — delete log files manually if you want to clear them.

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
- `bee/system/hash/deps` — hash of deps.yml
- `bee/system/hash/subsystems` — hash of the parent's subsystems registry

Each `bee/system/hash/<file>` entry has a companion `bee/system/hash/<file>.bee` containing the bee git hash (`bee version`) that created it, so future versions can tell which bee build produced each hash.

The system verifies integrity on every operation. If hashes don't match, bee returns "run bee init first".

---

## Automated Releases (GitHub Actions)

Bee uses `.github/workflows/release.yml` to build and publish precompiled binaries.

### Release conditions

For a **scheduled** release, a new release is created only when **both** conditions are true:

1. The current Git commit SHA differs from the commit SHA used for the last published Bee release.
2. At least one **calendar month** has passed since the last published Bee release (not 30 days).

For a **manual** release (`workflow_dispatch`), the calendar-month condition is skipped. Bee only checks that the previous published release does not have the same tag or release name (`bee-<full-commit-sha>` / `Bee <full-commit-sha>`). A duplicate name/tag results in a **pass** without building binaries.

If the applicable conditions are not met, the workflow finishes successfully with a **pass** result, does not build binaries, and does not create a release. The workflow logs clearly explain which condition was not satisfied.

If no previous published Bee release exists, the initial release is allowed when a valid commit SHA can be identified.

### Triggers

- **Schedule**: runs automatically once per day.
- **Manual**: run via GitHub Actions `workflow_dispatch` on the repository. Ignores the calendar-month condition; only duplicate release names/tags are rejected.

### Versioning

Releases use the full 40-character Git commit SHA:

- Tag: `bee-<full-commit-sha>`
- Release name: `Bee <full-commit-sha>`

The release points to the exact commit that was built.

### Supported platforms

| Platform | Architecture | Rust target | Archive |
|---|---|---|---|
| macOS | x64 | `x86_64-apple-darwin` | `.tar.gz` |
| macOS | ARM64 | `aarch64-apple-darwin` | `.tar.gz` |
| Linux | x64 | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Linux | ARM64 | `aarch64-unknown-linux-gnu` | `.tar.gz` |
| Windows | x64 | `x86_64-pc-windows-msvc` | `.zip` |
| FreeBSD | x64 | `x86_64-unknown-freebsd` | `.tar.gz` |

### Artifact naming

Artifacts are named:

- `bee-<full-commit-sha>-macos-x64.tar.gz`
- `bee-<full-commit-sha>-macos-arm64.tar.gz`
- `bee-<full-commit-sha>-linux-x64.tar.gz`
- `bee-<full-commit-sha>-linux-arm64.tar.gz`
- `bee-<full-commit-sha>-windows-x64.zip`
- `bee-<full-commit-sha>-freebsd-x64.tar.gz`

Each archive contains the `bee` executable (`bee.exe` on Windows) and a small `README.txt` with platform, target, commit SHA, and build date. A `SHA256SUMS` file is attached to the release.

### Verify checksums

After downloading an archive and `SHA256SUMS`:

```bash
sha256sum -c SHA256SUMS --ignore-missing
```

On macOS:

```bash
shasum -a 256 -c SHA256SUMS --ignore-missing
```

### Failure behavior

- GitHub API errors are treated as workflow errors, not as unmet release conditions.
- If any target fails to build or its artifact is missing, the release is not published.
- The workflow does not overwrite an existing release for the same tag.

## Updating Bee

`bee update` downloads the latest published Bee release from GitHub Releases instead of compiling from source.

```bash
bee update
```

What it does:
- Detects your platform and architecture.
- Finds the latest published release tagged `bee-<full-commit-sha>`.
- Downloads the matching release artifact and `SHA256SUMS`.
- Verifies the SHA-256 checksum when `SHA256SUMS` is available.
- Replaces the current `bee` executable in place.

Requirements:
- `curl` must be available for API/download requests.
- `tar` is required for `.tar.gz` archives; `unzip` for `.zip` archives.
- The current platform must be one of the supported release targets.

If no matching published Bee release or platform artifact is found, the command fails with an explicit error.
