use anyhow::Result;
use crate::parser::reader::config as reader;
use crate::file;

pub fn show_status() -> Result<()> {
    let pipelines = reader::read_pipelines()?;
    let tasks = reader::read_tasks_config()?;
    let rules = reader::read_rules_config()?;

    println!("[bee/info] Project status");
    println!("[bee/info]   Pipelines: {}", pipelines.len());
    println!("[bee/info]   Tasks:     {}", tasks.len());
    println!("[bee/info]   Rules:     {}", rules.len());
    println!();

    if !pipelines.is_empty() {
        println!("[bee/info] Pipelines:");
        for p in &pipelines {
            println!("[bee/info]   {}:", p.name);
            let content = file::get_file_content(&format!("bee/pipelines/{}.yml", p.name));
            if let Ok(c) = content {
                if let Ok(pipe) = reader::read_pipeline_from_string(&c) {
                    for task_name in &pipe.tasks {
                        let status = get_task_status(task_name);
                        println!("[bee/info]     {}{}", task_name, status);
                    }
                }
            }
        }
        println!();
    }

    if !tasks.is_empty() {
        println!("[bee/info] Tasks (cached / total):");
        let cached = count_cached_tasks(&tasks);
        println!("[bee/info]   {}/{} tasks cached", cached, tasks.len());
    }

    Ok(())
}

fn get_task_status(name: &str) -> String {
    let cache_path = format!("bee/cache/{}/last_run", name);
    if std::path::Path::new(&cache_path).exists() {
        if let Ok(hash) = std::fs::read_to_string(&cache_path) {
            if !hash.is_empty() {
                return format!(" [cached: {}]", &hash[..8]);
            }
        }
    }
    String::from(" [pending]")
}

fn count_cached_tasks(tasks: &[crate::parser::TaskConfig]) -> usize {
    tasks.iter().filter(|t| {
        let path = format!("bee/cache/{}/last_run", t.name);
        std::path::Path::new(&path).exists()
    }).count()
}
