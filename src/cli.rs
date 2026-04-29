use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bee")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(name = "run")]
    #[command(about = "Run a task", long_about = None)]
    Run {
        tag: String,

        #[arg(long)]
        no_cache: bool,

        #[arg(long)]
        only: bool
    }
}