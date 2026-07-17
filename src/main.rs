mod cli;
mod clean;
mod run;
mod init;
pub mod parser;
mod yaml;
mod file;
mod list;
mod add;
mod delete;
mod modify;
mod time;
mod hash;
mod system;
mod graph;
mod status;
mod install;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        cli::Commands::Clean => {
            if !file::check_bee_directory() {
                eprintln!("Error: run bee init first");
                return;
            }

            clean::clean_cache().unwrap_or_else(|e| eprintln!("Error: {}", e));
        },

        cli::Commands::Run => {
            if !file::check_bee_directory() {
                eprintln!("Error: run bee init first");
                return;
            }

            run::run_all().unwrap_or_else(|e| eprintln!("Error: {}", e));
        },

        cli::Commands::Init => {
            init::run_init().unwrap_or_else(|e| eprintln!("Error: {}", e));
        },

        cli::Commands::List => {
            if !file::check_bee_directory() {
                eprintln!("Error: run bee init first");
                return;
            }

            list::list_pipelines().unwrap_or_else(|e| eprintln!("Error: {}", e));
            list::list_tasks().unwrap_or_else(|e| eprintln!("Error: {}", e));
            list::list_rules().unwrap_or_else(|e| eprintln!("Error: {}", e));
        }

        cli::Commands::Status => {
            if !file::check_bee_directory() {
                eprintln!("Error: run bee init first");
                return;
            }

            status::show_status().unwrap_or_else(|e| eprintln!("Error: {}", e));
        }

        cli::Commands::Graph(graph_args) => {
            if !file::check_bee_directory() {
                eprintln!("Error: run bee init first");
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
                        Err(e) => eprintln!("Error: {}", e),
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
                        Err(e) => eprintln!("Error: {}", e),
                    }
                },
            }
        }

        cli::Commands::Backup(backup_args) => {
            if !file::check_bee_directory() {
                eprintln!("Error: run bee init first");
                return;
            }

            match backup_args.command {
                cli::BackupCommand::Create => {
                    system::backup::make_backup().unwrap_or_else(|e| eprintln!("Error: {}", e));
                },
                cli::BackupCommand::List => {
                    system::backup::list_backups().unwrap_or_else(|e| eprintln!("Error: {}", e));
                },
                cli::BackupCommand::Restore { hash } => {
                    system::backup::restore(&hash).unwrap_or_else(|e| eprintln!("Error: {}", e));
                },
            }
        }

        cli::Commands::Pipeline(pipeline_args) => {
            match pipeline_args.command {
                cli::PipelineCommand::Run { name } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    run::run_pipeline(name, None).unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::PipelineCommand::List => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                   list::list_pipelines().unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::PipelineCommand::Create { name, tasks } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    add::create_pipeline(&name, tasks).unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::PipelineCommand::Modify { name, add_tasks, rm_tasks } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    modify::modify_pipeline(&name, &add_tasks, &rm_tasks)
                        .unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::PipelineCommand::Delete { name } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    delete::delete_pipeline(&name).unwrap_or_else(|e| eprintln!("Error: {}", e));
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

                    run::run_task(name).unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::TaskCommand::List => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    list::list_tasks().unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::TaskCommand::Create { name, depends_on, run } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    add::create_task(&name, depends_on, run)
                        .unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::TaskCommand::Modify { name, add_deps, rm_deps, run } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    modify::modify_task(&name, &add_deps, &rm_deps, run.as_deref())
                        .unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::TaskCommand::Delete { name } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    delete::delete_task(&name).unwrap_or_else(|e| eprintln!("Error: {}", e));
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

                    list::list_rules().unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::RuleCommand::Create { name, task } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    add::create_rule(&name, &task).unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::RuleCommand::Modify { name, task, trigger_type, trigger_params } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    modify::modify_rule(&name, task.as_deref(), trigger_type.as_deref(), &trigger_params)
                        .unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::RuleCommand::Delete { name } => {
                    if !file::check_bee_directory() {
                        eprintln!("Error: run bee init first");
                        return;
                    }

                    delete::delete_rule(&name).unwrap_or_else(|e| eprintln!("Error: {}", e));
                }
            }
        }

        cli::Commands::Install(install_args) => {
            if !file::check_bee_directory() {
                eprintln!("Error: run bee init first");
                return;
            }

            match install_args.command {
                cli::InstallCommand::Run => {
                    install::install_all().unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::InstallCommand::List => {
                    install::list_deps().unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::InstallCommand::Add { name, dep_type, source, version, command } => {
                    install::add_dep(&name, &dep_type, &source, version.as_deref(), command.as_deref())
                        .unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::InstallCommand::Modify { name, dep_type, source, version, command } => {
                    modify::modify_dep(&name, dep_type.as_deref(), source.as_deref(), version.as_deref(), command.as_deref())
                        .unwrap_or_else(|e| eprintln!("Error: {}", e));
                },

                cli::InstallCommand::Remove { name } => {
                    install::remove_dep(&name).unwrap_or_else(|e| eprintln!("Error: {}", e));
                },
            }
        }
    }
}
