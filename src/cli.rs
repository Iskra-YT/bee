use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(
    name = "bee",
    about = "Task pipeline orchestration tool",
    long_about = "Bee manages task pipelines with automatic dependency resolution.\n\nDefine tasks in bee/tasks/*.yml, connect them via depends_on,\norganize into pipelines in bee/pipelines/*.yml, and control when\nthey run with trigger rules in bee/rules/*.yml.\n\nRun 'bee init' to get started in a new project."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize bee in the current directory
    #[command(about = "Initialize bee project structure (proposes a subsystem if a parent bee system is found)")]
    Init(InitArgs),

    /// Run all pipelines
    #[command(about = "Run all pipelines in dependency order")]
    Run,

    /// List all tasks, pipelines, and rules
    #[command(about = "List all registered tasks, pipelines, and rules")]
    List,

    /// Show project status and cached task counts
    #[command(about = "Show project status")]
    Status,

    /// Clear the runtime cache (trigger states, run history)
    #[command(about = "Clear cache and reset trigger states")]
    Clean,

    /// Update bee to the latest version from GitHub
    #[command(about = "Update bee to the latest version")]
    Update,

    /// Show the installed bee git hash (recorded at build/install time)
    #[command(about = "Show the installed bee git hash")]
    Version,

    /// Visualize dependency graphs
    #[command(about = "Render dependency graphs for pipelines")]
    Graph(GraphArgs),

    /// Manage pipeline definitions
    #[command(about = "Create, modify, run, and delete pipelines")]
    Pipeline(PipelineArgs),

    /// Manage task definitions
    #[command(about = "Create, modify, run, and delete tasks")]
    Task(TaskArgs),

    /// Manage trigger rules
    #[command(about = "Create, modify, and delete trigger rules")]
    Rule(RuleArgs),

    /// Manage external dependencies
    #[command(about = "Install, add, modify, and remove external dependencies")]
    Install(InstallArgs),

    /// Manage configuration backups
    #[command(about = "Create, list, and restore configuration backups")]
    Backup(BackupArgs),

    /// Manage bee subsystems
    #[command(about = "List and inspect bee subsystems linked to a parent system")]
    Sub(SubArgs),
}

// ─── Init ────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct InitArgs {
    /// Subsystem name (default: current directory name)
    #[arg(long, help = "Name for the subsystem (default: current directory name)")]
    pub name: Option<String>,

    /// Auto-confirm subsystem creation without prompting
    #[arg(short, long, help = "Create a subsystem without prompting")]
    pub yes: bool,
}

// ─── Sub ─────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct SubArgs {
    #[command(subcommand)]
    pub command: SubCommand,
}

#[derive(Subcommand)]
pub enum SubCommand {
    /// List subsystems
    #[command(about = "List subsystems registered in this project or in the parent system")]
    List,

    /// Show subsystem status
    #[command(about = "Show subsystem details and its link to the parent system")]
    Status,

    /// Run a task inside a subsystem
    #[command(about = "Invoke a task in a subsystem from the parent context")]
    Run {
        /// Subsystem name
        subsystem: String,

        /// Task name to run inside the subsystem
        task: String,
    },

    /// Remove a subsystem from the registry
    #[command(about = "Unregister a subsystem (optionally delete its directory)")]
    Remove {
        /// Subsystem name
        name: String,

        /// Also delete the subsystem directory
        #[arg(long, help = "Also delete the subsystem directory")]
        delete: bool,
    },

    /// Diagnose subsystem registration issues
    #[command(about = "Check registered subsystems for broken links, missing directories, and hash mismatches")]
    Doctor,

    /// Inspect dependencies inherited from the parent system (read-only)
    #[command(about = "List or resolve dependencies inherited from the parent system")]
    Deps(DepsArgs),
}

#[derive(Args)]
pub struct DepsArgs {
    #[command(subcommand)]
    pub command: DepsCommand,
}

#[derive(Subcommand)]
pub enum DepsCommand {
    /// List dependencies inherited from the parent system
    #[command(about = "List dependencies inherited from the parent system (read-only)")]
    List,

    /// Print the path to an inherited dependency
    #[command(about = "Print the path to a dependency installed by the parent system")]
    Path {
        /// Dependency name
        name: String,
    },
}

// ─── Task ───────────────────────────────────────────────────────────

#[derive(Args)]
pub struct TaskArgs {
    #[command(subcommand)]
    pub command: TaskCommand,
}

#[derive(Subcommand)]
pub enum TaskCommand {
    /// Run a single task
    #[command(about = "Execute a task directly (bypasses pipeline ordering)")]
    Run {
        /// Task name to run
        name: String,
    },

    /// Create a new task
    #[command(about = "Create a new task definition")]
    Create {
        /// Task name (used as filename: bee/tasks/<name>.yml)
        name: String,

        /// Add a dependency (repeatable: -d build -d test)
        #[arg(short, long = "dep", help = "Task this task depends on (repeatable)")]
        depends_on: Vec<String>,

        /// Shell command to execute (default: echo "Running <name>...")
        #[arg(short, long, help = "Shell command to run")]
        run: Option<String>,
    },

    /// Modify an existing task
    #[command(about = "Modify task dependencies or run command")]
    Modify {
        /// Task name to modify
        name: String,

        /// Add a dependency (repeatable)
        #[arg(long = "add-dep", help = "Add a dependency (repeatable)")]
        add_deps: Vec<String>,

        /// Remove a dependency (repeatable)
        #[arg(long = "rm-dep", help = "Remove a dependency (repeatable)")]
        rm_deps: Vec<String>,

        /// Replace the run command
        #[arg(short, long, help = "Replace the shell command")]
        run: Option<String>,
    },

    /// Delete a task
    #[command(about = "Delete a task definition")]
    Delete {
        /// Task name to delete
        name: String,
    },

    /// List all tasks
    #[command(about = "List all registered tasks")]
    List,
}

// ─── Pipeline ───────────────────────────────────────────────────────

#[derive(Args)]
pub struct PipelineArgs {
    #[command(subcommand)]
    pub command: PipelineCommand,
}

#[derive(Subcommand)]
pub enum PipelineCommand {
    /// Run a pipeline
    #[command(about = "Execute a pipeline (tasks run in DAG order, parallel where possible)")]
    Run {
        /// Pipeline name to run
        name: String,
    },

    /// Create a new pipeline
    #[command(about = "Create a new pipeline definition")]
    Create {
        /// Pipeline name (used as filename: bee/pipelines/<name>.yml)
        name: String,

        /// Add a task to the pipeline (repeatable: -t compile -t test)
        #[arg(short, long = "task", help = "Task to include (repeatable)")]
        tasks: Vec<String>,
    },

    /// Modify an existing pipeline
    #[command(about = "Add or remove tasks from a pipeline")]
    Modify {
        /// Pipeline name to modify
        name: String,

        /// Add a task (repeatable, skips duplicates)
        #[arg(long = "add-task", help = "Add a task (repeatable, skips duplicates)")]
        add_tasks: Vec<String>,

        /// Remove a task (repeatable)
        #[arg(long = "rm-task", help = "Remove a task (repeatable)")]
        rm_tasks: Vec<String>,
    },

    /// Delete a pipeline
    #[command(about = "Delete a pipeline definition")]
    Delete {
        /// Pipeline name to delete
        name: String,
    },

    /// List all pipelines
    #[command(about = "List all registered pipelines")]
    List,
}

// ─── Rule ───────────────────────────────────────────────────────────

#[derive(Args)]
pub struct RuleArgs {
    #[command(subcommand)]
    pub command: RuleCommand,
}

#[derive(Subcommand)]
pub enum RuleCommand {
    /// List all rules
    #[command(about = "List all registered trigger rules")]
    List,

    /// Create a new rule (default: manual trigger)
    #[command(about = "Create a new trigger rule for a task")]
    Create {
        /// Rule name (used as filename: bee/rules/<name>.yml)
        name: String,

        /// Target task name
        #[arg(short, long, help = "Task this rule applies to")]
        task: String,
    },

    /// Modify an existing rule
    #[command(about = "Modify rule task or add triggers")]
    Modify {
        /// Rule name to modify
        name: String,

        /// Change the target task
        #[arg(short, long, help = "Change the target task")]
        task: Option<String>,

        /// Add a trigger type (see available types below)
        #[arg(long = "trigger-type", help = "Trigger type: manual, always, file_change, git_changed, task_completed, task_failed, dependency_changed, env_changed, dependency_missing, checksum_changed, schedule, git_tag, subtask_completed, subtask_failed, subtask_changed, parent_task_completed, parent_task_failed, subsystem_changed")]
        trigger_type: Option<String>,

        /// Trigger parameters as key=value pairs (repeatable)
        #[arg(long = "trigger-param", help = "Param for the trigger (repeatable): paths=glob1,glob2 | task=name | subsystem=name | vars=VAR1,VAR2 | path=/file | cron=expr | pattern=regex")]
        trigger_params: Vec<String>,
    },

    /// Delete a rule
    #[command(about = "Delete a trigger rule")]
    Delete {
        /// Rule name to delete
        name: String,
    },
}

// ─── Install ────────────────────────────────────────────────────────

#[derive(Args)]
pub struct InstallArgs {
    #[command(subcommand)]
    pub command: InstallCommand,
}

#[derive(Subcommand)]
pub enum InstallCommand {
    /// Install all dependencies
    #[command(about = "Install all dependencies defined in system/deps.yml")]
    Run,

    /// Update all installed dependencies
    #[command(about = "Update all installed dependencies (fetches latest changes)")]
    Update,

    /// List all dependencies
    #[command(about = "List all defined dependencies")]
    List,

    /// Add a dependency
    #[command(about = "Add a new external dependency")]
    Add {
        /// Dependency name
        name: String,

        /// Dependency type: package, url, git, command, or manager
        #[arg(short, long, help = "Type: package (apt/brew), url (archive download), git (clone repo), command (shell command), manager (package manager in path)")]
        dep_type: String,

        /// Source: package name, URL, or git repo URL (not required for command/manager types)
        #[arg(short, long, help = "Source: package name, download URL, or git repo URL (optional for type=command/manager)")]
        source: Option<String>,

        /// Version (optional, for url/git types)
        #[arg(short, long, help = "Version tag (optional, for url/git)")]
        version: Option<String>,

        /// Shell command to execute (required for type=command, optional for type=package/manager)
        #[arg(short, long, help = "Shell command (required for type=command, optional for type=manager)")]
        command: Option<String>,

        /// Install script content (optional, for url/git types). Written to deps/<name>/install.sh and executed after download/clone
        #[arg(long = "install-script", help = "Install script content (written to deps/<name>/install.sh, run after download/clone)")]
        install_script: Option<String>,

        /// Working directory for manager type (required for type=manager)
        #[arg(short, long, help = "Working directory path (required for type=manager, e.g. src/frontend)")]
        path: Option<String>,
    },

    /// Modify an existing dependency
    #[command(about = "Modify dependency source, version, or command")]
    Modify {
        /// Dependency name to modify
        name: String,

        /// Change the dependency type
        #[arg(short, long, help = "New type: package, url, git, command, or manager")]
        dep_type: Option<String>,

        /// Change the source
        #[arg(short, long, help = "New source URL or package name")]
        source: Option<String>,

        /// Change the version
        #[arg(short, long, help = "New version tag")]
        version: Option<String>,

        /// Change the install command
        #[arg(short, long, help = "New install command")]
        command: Option<String>,

        /// Change the install script content
        #[arg(long = "install-script", help = "New install script content (written to deps/<name>/install.sh)")]
        install_script: Option<String>,

        /// Change the working directory path (for manager type)
        #[arg(short, long, help = "New working directory path (for type=manager)")]
        path: Option<String>,
    },

    /// Remove a dependency
    #[command(about = "Remove a dependency and its installed files")]
    Remove {
        /// Dependency name to remove
        name: String,
    },
}

// ─── Graph ──────────────────────────────────────────────────────────

#[derive(Args)]
pub struct GraphArgs {
    #[command(subcommand)]
    pub command: GraphCommand,
}

#[derive(Subcommand)]
pub enum GraphCommand {
    /// Show dependency graphs for all pipelines
    #[command(about = "Render dependency graphs for all pipelines")]
    All {
        /// Output format: tree (default), dot, or mermaid
        #[arg(default_value = "tree", help = "Format: tree (text), dot (GraphViz), mermaid")]
        format: String,
    },

    /// Show dependency graph for a specific pipeline
    #[command(about = "Render dependency graph for one pipeline")]
    Pipeline {
        /// Pipeline name
        name: String,

        /// Output format: tree (default), dot, or mermaid
        #[arg(default_value = "tree", help = "Format: tree (text), dot (GraphViz), mermaid")]
        format: String,
    },
}

// ─── Backup ─────────────────────────────────────────────────────────

#[derive(Args)]
pub struct BackupArgs {
    #[command(subcommand)]
    pub command: BackupCommand,
}

#[derive(Subcommand)]
pub enum BackupCommand {
    /// Create a backup of tasks, pipelines, rules, and config
    #[command(about = "Snapshot current configuration to bee/system/backup/")]
    Create,

    /// List all backups
    #[command(about = "List available backups with dates and hashes")]
    List,

    /// Restore a backup by hash
    #[command(about = "Restore tasks, pipelines, rules, and config from a backup")]
    Restore {
        /// Backup hash (from 'bee backup list')
        hash: String,
    },
}
