// Reuse the library crate instead of compiling a second copy of these modules
// into the binary; otherwise items only used by the library/tests look dead.
use bee::{
    add, clean, cli, delete, docs, file, graph, hash, init, inspect, install, list, modify, run,
    schedule, status, subsystem, system, ui, update,
};

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() {
    if let Err(error) = run() {
        let exit_code = run::tasks::failure_exit_code(&error).unwrap_or(1);
        eprintln!("Error: {:#}", error);
        std::process::exit(exit_code);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    ui::configure(cli.no_color, cli.dry_run, cli.json);

    match cli.command {
        cli::Commands::Clean => {
            file::ensure_bee_directory()?;
            clean::clean_cache()?;
        }

        cli::Commands::Update => {
            update::run_update()?;
        }

        cli::Commands::Version => {
            println!("{}", hash::bee_git_hash());
        }

        cli::Commands::Run => {
            file::ensure_bee_directory()?;
            if ui::dry_run() {
                run::dry_run_all()?;
            } else {
                run::run_all()?;
            }
        }

        cli::Commands::Init(args) => {
            init::run_init(args.name.as_deref(), args.yes)?;
        }

        cli::Commands::List => {
            file::ensure_bee_directory()?;
            if ui::json_mode() {
                ui::print_json(&inspect::list_json()?);
            } else {
                list::list_pipelines()?;
                list::list_tasks()?;
                list::list_rules()?;
            }
        }

        cli::Commands::Status => {
            file::ensure_bee_directory()?;
            if ui::json_mode() {
                ui::print_json(&inspect::status_json()?);
            } else {
                status::show_status()?;
            }
        }

        cli::Commands::Schedule(args) => {
            file::ensure_bee_directory()?;
            match args.command {
                cli::ScheduleCommand::Run { interval, once } => {
                    schedule::run_daemon(interval, once)?;
                }
                cli::ScheduleCommand::List => schedule::list_scheduled()?,
            }
        }

        cli::Commands::Validate => {
            file::ensure_bee_directory()?;
            inspect::validate_command()?;
        }

        cli::Commands::Logs(args) => {
            file::ensure_bee_directory()?;
            match args.command {
                cli::LogsCommand::Task { name } => inspect::logs_task_command(&name)?,
                cli::LogsCommand::List => inspect::logs_list_command()?,
            }
        }

        cli::Commands::Show(args) => {
            file::ensure_bee_directory()?;
            match args.command {
                cli::ShowCommand::Task { name } => inspect::show_command("task", &name)?,
                cli::ShowCommand::Pipeline { name } => inspect::show_command("pipeline", &name)?,
                cli::ShowCommand::Rule { name } => inspect::show_command("rule", &name)?,
            }
        }

        cli::Commands::Completions(args) => {
            use clap::CommandFactory;
            let shell = match args.shell.as_str() {
                "bash" => clap_complete::Shell::Bash,
                "zsh" => clap_complete::Shell::Zsh,
                "fish" => clap_complete::Shell::Fish,
                "powershell" => clap_complete::Shell::PowerShell,
                other => return Err(anyhow::anyhow!("unsupported shell '{}'", other)),
            };
            clap_complete::generate(shell, &mut Cli::command(), "bee", &mut std::io::stdout());
        }

        cli::Commands::Graph(args) => {
            file::ensure_bee_directory()?;
            match args.command {
                cli::GraphCommand::All { format } => {
                    let output = graph::render_all_pipelines(graph::parse_format(&format)?)?;
                    println!("{}", output);
                }
                cli::GraphCommand::Pipeline { name, format } => {
                    let output =
                        graph::render_pipeline_graph(&name, graph::parse_format(&format)?)?;
                    println!("{}", output);
                }
            }
        }

        cli::Commands::Backup(args) => {
            file::ensure_bee_directory()?;
            if file::is_subsystem() {
                return Err(anyhow::anyhow!("backup is not available in a subsystem"));
            }
            match args.command {
                cli::BackupCommand::Create => system::backup::make_backup()?,
                cli::BackupCommand::List => system::backup::list_backups()?,
                cli::BackupCommand::Restore { hash } => system::backup::restore(&hash)?,
            }
        }

        cli::Commands::Docs(args) => {
            file::ensure_bee_directory()?;
            match args.command {
                cli::DocsCommand::Update => docs::update_docs()?,
            }
        }

        cli::Commands::Subsystem(args) => {
            file::ensure_bee_directory()?;
            match args.command {
                cli::SubsystemCommand::List => subsystem::subsystem_list()?,
                cli::SubsystemCommand::Status => subsystem::subsystem_status()?,
                cli::SubsystemCommand::Run { subsystem, target } => match target {
                    cli::SubsystemRunTarget::Task { task } => {
                        subsystem::subsystem_run_task(&subsystem, &task)?;
                    }
                    cli::SubsystemRunTarget::Pipeline { pipeline } => {
                        subsystem::subsystem_run_pipeline(&subsystem, &pipeline)?;
                    }
                },
                cli::SubsystemCommand::Remove { name, delete } => {
                    subsystem::subsystem_remove(&name, delete)?;
                }
                cli::SubsystemCommand::Doctor => subsystem::subsystem_doctor()?,
                cli::SubsystemCommand::Deps(args) => match args.command {
                    cli::DepsCommand::List => subsystem::subsystem_deps_list()?,
                    cli::DepsCommand::Path { name } => subsystem::subsystem_deps_path(&name)?,
                },
            }
        }

        cli::Commands::Pipeline(args) => match args.command {
            cli::PipelineCommand::Run { name } => {
                file::ensure_bee_directory()?;
                if ui::dry_run() {
                    run::dry_run_pipeline(&name)?;
                } else {
                    run::run_pipeline(name, None)?;
                }
            }
            cli::PipelineCommand::List => {
                file::ensure_bee_directory()?;
                list::list_pipelines()?;
            }
            cli::PipelineCommand::Create {
                name,
                tasks,
                pipelines,
            } => {
                file::ensure_bee_directory()?;
                add::create_pipeline(&name, tasks, pipelines)?;
            }
            cli::PipelineCommand::Modify {
                name,
                add_tasks,
                rm_tasks,
                add_pipelines,
                rm_pipelines,
            } => {
                file::ensure_bee_directory()?;
                modify::modify_pipeline(
                    &name,
                    &add_tasks,
                    &rm_tasks,
                    &add_pipelines,
                    &rm_pipelines,
                )?;
            }
            cli::PipelineCommand::Delete { name } => {
                file::ensure_bee_directory()?;
                delete::delete_pipeline(&name)?;
            }
        },

        cli::Commands::Task(args) => match args.command {
            cli::TaskCommand::Run { name, force } => {
                file::ensure_bee_directory()?;
                if ui::dry_run() {
                    run::dry_run_task(&name)?;
                } else {
                    run::run_task(name, force, true)?;
                }
            }
            cli::TaskCommand::List => {
                file::ensure_bee_directory()?;
                list::list_tasks()?;
            }
            cli::TaskCommand::Create {
                name,
                depends_on,
                run,
                executor,
                program,
                args,
                working_dir,
                shell,
                interpreter,
                script,
            } => {
                file::ensure_bee_directory()?;
                add::create_task(
                    &name,
                    depends_on,
                    run,
                    executor,
                    program,
                    args,
                    working_dir,
                    shell,
                    interpreter,
                    script,
                )?;
            }
            cli::TaskCommand::Modify {
                name,
                add_deps,
                rm_deps,
                run,
                executor,
                program,
                args,
                working_dir,
                shell,
                interpreter,
                script,
            } => {
                file::ensure_bee_directory()?;
                modify::modify_task(
                    &name,
                    &add_deps,
                    &rm_deps,
                    run.as_deref(),
                    executor.as_deref(),
                    program.as_deref(),
                    &args,
                    working_dir.as_deref(),
                    shell.as_deref(),
                    interpreter.as_deref(),
                    script.as_deref(),
                )?;
            }
            cli::TaskCommand::Delete { name } => {
                file::ensure_bee_directory()?;
                delete::delete_task(&name)?;
            }
        },

        cli::Commands::Rule(args) => match args.command {
            cli::RuleCommand::List => {
                file::ensure_bee_directory()?;
                list::list_rules()?;
            }
            cli::RuleCommand::Create { name, task } => {
                file::ensure_bee_directory()?;
                add::create_rule(&name, &task)?;
            }
            cli::RuleCommand::Modify {
                name,
                task,
                trigger_type,
                trigger_params,
            } => {
                file::ensure_bee_directory()?;
                modify::modify_rule(
                    &name,
                    task.as_deref(),
                    trigger_type.as_deref(),
                    &trigger_params,
                )?;
            }
            cli::RuleCommand::Delete { name } => {
                file::ensure_bee_directory()?;
                delete::delete_rule(&name)?;
            }
        },

        cli::Commands::Install(args) => {
            file::ensure_bee_directory()?;
            if file::is_subsystem() {
                return Err(anyhow::anyhow!("install is not available in a subsystem"));
            }
            match args.command {
                cli::InstallCommand::Run => install::install_all()?,
                cli::InstallCommand::Update => install::update_all()?,
                cli::InstallCommand::List => install::list_deps()?,
                cli::InstallCommand::Add {
                    name,
                    dep_type,
                    source,
                    version,
                    command,
                    install_script,
                    path,
                } => {
                    install::add_dep(
                        &name,
                        &dep_type,
                        source.as_deref(),
                        version.as_deref(),
                        command.as_deref(),
                        install_script.as_deref(),
                        path.as_deref(),
                    )?;
                }
                cli::InstallCommand::Modify {
                    name,
                    dep_type,
                    source,
                    version,
                    command,
                    install_script,
                    path,
                } => {
                    modify::modify_dep(
                        &name,
                        dep_type.as_deref(),
                        source.as_deref(),
                        version.as_deref(),
                        command.as_deref(),
                        install_script.as_deref(),
                        path.as_deref(),
                    )?;
                }
                cli::InstallCommand::Remove { name } => install::remove_dep(&name)?,
            }
        }
    }

    Ok(())
}
