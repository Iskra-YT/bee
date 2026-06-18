use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(name = "bee")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Run,
    Init,
    List,
    Status,
    Graph(GraphArgs),
    Backup,
    Pipeline(PipelineArgs),
    Task(TaskArgs),
    Rule(RuleArgs)
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
    }
}