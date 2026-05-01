mod cli;
mod run;
mod init;
mod parser;
mod yaml;
mod file;
mod list;

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
                },

                cli::PipelineCommand::List => {
                   list::list_pipelines();
                }
            }
        },

        cli::Commands::Task(task_args) => {
            match task_args.command {
                cli::TaskCommand::Run { name } => {
                    run::run_task(name);
                }

                cli::TaskCommand::List => {
                    list::list_tasks();
                }
            }
        }

        cli::Commands::Rule(rule_args) => {
            match rule_args.command {
                cli::RuleCommand::List => {
                    list::list_rules();
                }
            }
        }
    }
}
