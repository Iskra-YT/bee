mod cli;
mod run;
mod init;
mod parser;
mod yaml;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        cli::Commands::Run {  } => {
            run::run_all();
        },

        cli::Commands::Init => {
            init::run_init().unwrap();
            println!("Initialization complete!");
        },

        cli::Commands::Pipeline(pipeline_args) => {
            match pipeline_args.command {
                cli::PipelineCommand::Run { name } => {
                    run::run_pipeline(name);
                }
            }
        },

        cli::Commands::Task(task_args) => {
            match task_args.command {
                cli::TaskCommand::Run { name } => {
                    run::run_task(name);
                }
            }
        }
    }
}
