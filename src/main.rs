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
mod system;
mod graph;
mod status;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        cli::Commands::Run => {
            if !file::check_bee_directory() {
                eprintln!("[bee/error] Error: run bee init first");
                return;
            }

            run::run_all().unwrap_or_else(|e| eprintln!("[bee/error] Error running pipelines: {}", e));
        },

        cli::Commands::Init => {
            init::run_init().unwrap_or_else(|e| eprintln!("[bee/error] Error initializing bee: {}", e));
        },

        cli::Commands::List => {
            if !file::check_bee_directory() {
                eprintln!("[bee/error] Error: run bee init first");
                return;
            }

            list::list_pipelines().unwrap_or_else(|e| eprintln!("[bee/error] Error listing pipelines: {}", e));
            println!("[bee/info] ");
            list::list_tasks().unwrap_or_else(|e| eprintln!("[bee/error] Error listing tasks: {}", e));
            println!("[bee/info] ");
            list::list_rules().unwrap_or_else(|e| eprintln!("[bee/error] Error listing rules: {}", e));
        }

        cli::Commands::Status => {
            if !file::check_bee_directory() {
                eprintln!("[bee/error] Error: run bee init first");
                return;
            }

            status::show_status().unwrap_or_else(|e| eprintln!("[bee/error] Error showing status: {}", e));
        }

        cli::Commands::Graph(graph_args) => {
            if !file::check_bee_directory() {
                eprintln!("[bee/error] Error: run bee init first");
                return;
            }

            match graph_args.command {
                cli::GraphCommand::All { format } => {
                    let fmt = match format.as_str() {
                        "dot" => graph::GraphFormat::Dot,
                        "mermaid" => graph::GraphFormat::Mermaid,
                        _ => graph::GraphFormat::Tree,
                    };
                    match graph::render_all_pipelines(fmt) {
                        Ok(output) => println!("{}", output),
                        Err(e) => eprintln!("[bee/error] Error rendering graph: {}", e),
                    }
                },
                cli::GraphCommand::Pipeline { name, format } => {
                    let fmt = match format.as_str() {
                        "dot" => graph::GraphFormat::Dot,
                        "mermaid" => graph::GraphFormat::Mermaid,
                        _ => graph::GraphFormat::Tree,
                    };
                    match graph::render_pipeline_graph(&name, fmt) {
                        Ok(output) => println!("{}", output),
                        Err(e) => eprintln!("[bee/error] Error rendering graph: {}", e),
                    }
                },
            }
        }

        cli::Commands::Backup(backup_args) => {
            if !file::check_bee_directory() {
                eprintln!("[bee/error] Error: run bee init first");
                return;
            }

            match backup_args.command {
                cli::BackupCommand::Create => {
                    system::backup::make_backup().unwrap_or_else(|e| eprintln!("[bee/error] Error creating backup: {}", e));
                },
                cli::BackupCommand::List => {
                    system::backup::list_backups().unwrap_or_else(|e| eprintln!("[bee/error] Error listing backups: {}", e));
                },
                cli::BackupCommand::Restore { hash } => {
                    system::backup::restore(&hash).unwrap_or_else(|e| eprintln!("[bee/error] Error restoring backup: {}", e));
                },
            }
        }

        cli::Commands::Pipeline(pipeline_args) => {
            match pipeline_args.command {
                cli::PipelineCommand::Run { name } => {
                    if !file::check_bee_directory() {
                        eprintln!("[bee/error] Error: run bee init first");
                        return;
                    }

                    run::run_pipeline(name, None).unwrap_or_else(|e| eprintln!("[bee/error] Error running pipeline: {}", e));
                },

                cli::PipelineCommand::List => {
                    if !file::check_bee_directory() {
                        eprintln!("[bee/error] Error: run bee init first");
                        return;
                    }

                   list::list_pipelines().unwrap_or_else(|e| eprintln!("[bee/error] Error listing pipelines: {}", e));
                },

                cli::PipelineCommand::Create { name } => {
                    if !file::check_bee_directory() {
                        eprintln!("[bee/error] Error: run bee init first");
                        return;
                    }

                    add::create_pipeline(&name).unwrap_or_else(|e| eprintln!("[bee/error] Error creating pipeline: {}", e));
                }
            }
        },

        cli::Commands::Task(task_args) => {
            match task_args.command {
                cli::TaskCommand::Run { name } => {
                    if !file::check_bee_directory() {
                        eprintln!("[bee/error] Error: run bee init first");
                        return;
                    }

                    run::run_task(name).unwrap_or_else(|e| eprintln!("[bee/error] Error running task: {}", e));
                },

                cli::TaskCommand::List => {
                    if !file::check_bee_directory() {
                        eprintln!("[bee/error] Error: run bee init first");
                        return;
                    }

                    list::list_tasks().unwrap_or_else(|e| eprintln!("[bee/error] Error listing tasks: {}", e));
                },

                cli::TaskCommand::Create { name } => {
                    if !file::check_bee_directory() {
                        eprintln!("[bee/error] Error: run bee init first");
                        return;
                    }

                    add::create_task(&name).unwrap_or_else(|e| eprintln!("[bee/error] Error creating task: {}", e));
                }
            }
        }

        cli::Commands::Rule(rule_args) => {
            match rule_args.command {
                cli::RuleCommand::List => {
                    if !file::check_bee_directory() {
                        eprintln!("[bee/error] Error: run bee init first");
                        return;
                    }

                    list::list_rules().unwrap_or_else(|e| eprintln!("[bee/error] Error listing rules: {}", e));
                },

                cli::RuleCommand::Create { name } => {
                    if !file::check_bee_directory() {
                        eprintln!("[bee/error] Error: run bee init first");
                        return;
                    }

                    add::create_rule(&name).unwrap_or_else(|e| eprintln!("[bee/error] Error creating rule: {}", e));
                }
            }
        }
    }
}
