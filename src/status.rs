use crate::file;
use crate::parser::reader::config as reader;
use crate::run::cache;
use anyhow::Result;

pub fn show_status() -> Result<()> {
    let pipelines = reader::read_pipelines()?;
    let tasks = reader::read_tasks_config()?;
    let rules = reader::read_rules_config()?;

    println!("Project status");
    println!("  Pipelines: {}", pipelines.len());
    println!("  Tasks:     {}", tasks.len());
    println!("  Rules:     {}", rules.len());
    println!();

    if !pipelines.is_empty() {
        println!("Pipelines:");
        for p in &pipelines {
            println!("  {}:", p.name);
            let content = file::get_file_content(&format!("bee/pipelines/{}.yml", p.name));
            if let Ok(c) = content {
                if let Ok(pipe) = reader::read_pipeline_from_string(&c) {
                    if !pipe.pipelines.is_empty() {
                        println!("    subpipelines: {}", pipe.pipelines.join(", "));
                    }
                    for task_name in &pipe.tasks {
                        println!("    {}{}", task_name, get_task_status(task_name));
                    }
                }
            }
        }
        println!();
    }

    if !tasks.is_empty() {
        println!("Tasks (cached / total):");
        let cached = count_cached_tasks(&tasks);
        println!("  {}/{} tasks cached", cached, tasks.len());
    }

    Ok(())
}

fn get_task_status(name: &str) -> String {
    match std::fs::read_to_string(cache::last_run_path(name)) {
        Ok(hash) if !hash.trim().is_empty() => {
            let short: String = hash.trim().chars().take(8).collect();
            format!(" [cached: {}]", short)
        }
        _ => String::from(" [pending]"),
    }
}

fn count_cached_tasks(tasks: &[crate::parser::TaskConfig]) -> usize {
    tasks
        .iter()
        .filter(|t| {
            std::fs::read_to_string(cache::last_run_path(&t.name))
                .map(|content| !content.trim().is_empty())
                .unwrap_or(false)
        })
        .count()
}
