use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(name = "bee")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Clean,
    Run,
    Init,
    List,
    Status,
    Graph(GraphArgs),
    Backup(BackupArgs),
    Pipeline(PipelineArgs),
    Task(TaskArgs),
    Rule(RuleArgs),
    Install(InstallArgs),
}

#[derive(Args)]
pub struct BackupArgs {
    #[command(subcommand)]
    pub command: BackupCommand,
}

#[derive(Subcommand)]
pub enum BackupCommand {
    Create,
    List,
    Restore {
        hash: String,
    },
}

#[derive(Args)]
pub struct GraphArgs {
    #[command(subcommand)]
    pub command: GraphCommand,
}

#[derive(Subcommand)]
pub enum GraphCommand {
    All {
        #[arg(default_value = "tree")]
        format: String,
    },
    Pipeline {
        name: String,
        #[arg(default_value = "tree")]
        format: String,
    },
}

#[derive(Args)]
pub struct PipelineArgs {
    #[command(subcommand)]
    pub command: PipelineCommand,
}

#[derive(Subcommand)]
pub enum PipelineCommand {
    Run {
        name: String,
    },

    Create {
        name: String,

        #[arg(short, long = "task")]
        tasks: Vec<String>,
    },

    Modify {
        name: String,

        #[arg(long = "add-task")]
        add_tasks: Vec<String>,

        #[arg(long = "rm-task")]
        rm_tasks: Vec<String>,
    },

    Delete {
        name: String,
    },

    List,
}

#[derive(Args)]
pub struct TaskArgs {
    #[command(subcommand)]
    pub command: TaskCommand,
}

#[derive(Subcommand)]
pub enum TaskCommand {
    Run {
        name: String,
    },

    Create {
        name: String,

        #[arg(short, long = "dep")]
        depends_on: Vec<String>,

        #[arg(short, long)]
        run: Option<String>,
    },

    Modify {
        name: String,

        #[arg(long = "add-dep")]
        add_deps: Vec<String>,

        #[arg(long = "rm-dep")]
        rm_deps: Vec<String>,

        #[arg(short, long)]
        run: Option<String>,
    },

    Delete {
        name: String,
    },

    List,
}

#[derive(Args)]
pub struct RuleArgs {
    #[command(subcommand)]
    pub command: RuleCommand,
}

#[derive(Subcommand)]
pub enum RuleCommand {
    List,

    Create {
        name: String,
        #[arg(short, long)]
        task: String,
    },

    Modify {
        name: String,

        #[arg(short, long)]
        task: Option<String>,

        #[arg(long = "trigger-type")]
        trigger_type: Option<String>,

        #[arg(long = "trigger-param")]
        trigger_params: Vec<String>,
    },

    Delete {
        name: String,
    },
}

#[derive(Args)]
pub struct InstallArgs {
    #[command(subcommand)]
    pub command: InstallCommand,
}

#[derive(Subcommand)]
pub enum InstallCommand {
    Run,
    List,
    Add {
        name: String,
        #[arg(short, long)]
        dep_type: String,
        #[arg(short, long)]
        source: String,
        #[arg(short, long)]
        version: Option<String>,
        #[arg(short, long)]
        command: Option<String>,
    },
    Modify {
        name: String,
        #[arg(short, long)]
        dep_type: Option<String>,
        #[arg(short, long)]
        source: Option<String>,
        #[arg(short, long)]
        version: Option<String>,
        #[arg(short, long)]
        command: Option<String>,
    },
    Remove {
        name: String,
    },
}
