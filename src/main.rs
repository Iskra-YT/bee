mod cli;
mod run;
mod init;
mod parser;
mod yaml;
mod file;
mod list;
mod add;

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

        cli::Commands::List => {
            list::list_pipelines().unwrap_or_else(|e| eprint!("Error listing pipelines: {}", e));
            println!("");
            list::list_tasks().unwrap_or_else(|e| eprint!("Error listing tasks: {}", e));
            println!("");
            list::list_rules().unwrap_or_else(|e| eprint!("Error listing rules: {}", e));
        }

        cli::Commands::Pipeline(pipeline_args) => {
            match pipeline_args.command {
                cli::PipelineCommand::Run { name } => {
                    run::run_pipeline(name);
                },

                cli::PipelineCommand::List => {
                   list::list_pipelines().unwrap_or_else(|e| eprint!("Error listing pipelines: {}", e));
                },

                cli::PipelineCommand::Add { name } => {
                    add::create_pipeline(&name).unwrap_or_else(|e| eprint!("Error creating pipeline: {}", e));
                }
            }
        },

        cli::Commands::Task(task_args) => {
            match task_args.command {
                cli::TaskCommand::Run { name } => {
                    run::run_task(name);
                }

                cli::TaskCommand::List => {
                    list::list_tasks().unwrap_or_else(|e| eprint!("Error listing tasks: {}", e));
                }

                cli::TaskCommand::Add { name } => {
                    add::create_task(&name).unwrap_or_else(|e| eprint!("Error creating task: {}", e));
                }
            }
        }

        cli::Commands::Rule(rule_args) => {
            match rule_args.command {
                cli::RuleCommand::List => {
                    list::list_rules().unwrap_or_else(|e| eprint!("Error listing rules: {}", e));
                },

                cli::RuleCommand::Add { name } => {
                    add::create_rule(&name).unwrap_or_else(|e| eprint!("Error creating rule: {}", e));
                }
            }
        }
    }
}
