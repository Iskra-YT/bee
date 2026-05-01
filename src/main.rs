mod cli;
mod run;
mod init;
pub mod parser;
mod yaml;
mod file;
mod list;
mod add;
mod time;
mod hash;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        cli::Commands::Run => {
            if !file::check_bee_directory() {
                eprintln!("Error: run bee init first");
                return;
            }

            run::run_all().unwrap_or_else(|e| eprintln!("Error running pipelines: {}", e));
        },

        cli::Commands::Init => {
            init::run_init().unwrap();
        },

        cli::Commands::List => {
            if !file::check_bee_directory() {
                eprintln!("Error: run bee init first");
                return;
            }

            list::list_pipelines().unwrap_or_else(|e| eprintln!("Error listing pipelines: {}", e));
            println!("");
            list::list_tasks().unwrap_or_else(|e| eprintln!("Error listing tasks: {}", e));
            println!("");
            list::list_rules().unwrap_or_else(|e| eprintln!("Error listing rules: {}", e));
        }

        cli::Commands::Pipeline(pipeline_args) => {
            match pipeline_args.command {
                cli::PipelineCommand::Run { name } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    run::run_pipeline(name, None).unwrap_or_else(|e| eprintln!("Error running pipeline: {}", e));
                },

                cli::PipelineCommand::List => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                   list::list_pipelines().unwrap_or_else(|e| eprintln!("Error listing pipelines: {}", e));
                },

                cli::PipelineCommand::Add { name } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    add::create_pipeline(&name).unwrap_or_else(|e| eprintln!("Error creating pipeline: {}", e));
                }
            }
        },

        cli::Commands::Task(task_args) => {
            match task_args.command {
                cli::TaskCommand::Run { name } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    run::run_task(name).unwrap_or_else(|e| eprintln!("Error running task: {}", e));
                },

                cli::TaskCommand::List => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    list::list_tasks().unwrap_or_else(|e| eprintln!("Error listing tasks: {}", e));
                },

                cli::TaskCommand::Add { name } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    add::create_task(&name).unwrap_or_else(|e| eprintln!("Error creating task: {}", e));
                }
            }
        }

        cli::Commands::Rule(rule_args) => {
            match rule_args.command {
                cli::RuleCommand::List => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    list::list_rules().unwrap_or_else(|e| eprintln!("Error listing rules: {}", e));
                },

                cli::RuleCommand::Add { name } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    add::create_rule(&name).unwrap_or_else(|e| eprintln!("Error creating rule: {}", e));
                }
            }
        }
    }
}
