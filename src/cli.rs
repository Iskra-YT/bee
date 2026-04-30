use clap::{Parser, Subcommand, ArgAction};

#[derive(Parser)]
#[command(name = "bee")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(name = "run")]
    Run {
        tag: String,

        #[arg(short = 't', long = "task", action = ArgAction::SetTrue, required = false)]
        task: bool,
    },
    
    #[command(name = "init")]
    Init,
}