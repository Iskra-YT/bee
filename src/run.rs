pub mod cache;
pub mod dag;
mod parallel;
mod pipeline;
pub mod tasks;

use crate::parser::reader::config as parser;
use anyhow::Result;

pub fn run_pipeline(name: String, config: Option<crate::parser::Pipeline>) -> Result<()> {
    if let Some(config) = config {
        pipeline::run_pipeline(config)?;
    } else {
        let pipelines = parser::read_pipelines()?;
        if let Some(pipeline) = pipelines.into_iter().find(|p| p.name == name) {
            pipeline::run_pipeline(pipeline)?;
        } else {
            return Err(anyhow::anyhow!("pipeline '{}' not found", name));
        }
    }

    Ok(())
}

pub fn run_all() -> Result<()> {
    let pipelines = parser::read_pipelines()?;

    for pipeline in pipelines {
        run_pipeline(pipeline.name.clone(), Some(pipeline))?;
    }

    Ok(())
}

pub fn run_task(name: String, force: bool, interactive: bool) -> Result<()> {
    let task_file = crate::safety::safe_path("bee/tasks", "task", &name)?.with_extension("yml");
    let content = crate::file::get_file_content(&task_file.to_string_lossy())
        .map_err(|_| anyhow::anyhow!("task '{}' not found; run 'bee task list'", name))?;
    let task = parser::read_task_from_string(&content, &name)?;

    tasks::run_task(task, force, interactive)
}

/// Print the work a task run would perform without executing anything.
pub fn dry_run_task(name: &str) -> Result<()> {
    let task_file = crate::safety::safe_path("bee/tasks", "task", name)?.with_extension("yml");
    let content = crate::file::get_file_content(&task_file.to_string_lossy())
        .map_err(|_| anyhow::anyhow!("task '{}' not found; run 'bee task list'", name))?;
    let task = parser::read_task_from_string(&content, name)?;
    crate::ui::info(format!(
        "[dry-run] would run task '{}' (executor: {})",
        task.name,
        task.effective_executor()
    ));
    Ok(())
}

/// Print DAG layers for a pipeline without executing tasks.
pub fn dry_run_pipeline(name: &str) -> Result<()> {
    let pipeline = parser::read_pipeline(name)?;
    let (order, graph) = dag::build_dag(pipeline.clone())?;
    let groups = parallel::find_parallel_groups(&graph, &order);
    let total: usize = groups.iter().map(Vec::len).sum();
    crate::ui::info(format!(
        "[dry-run] pipeline '{}': {} task(s) in {} group(s)",
        name,
        total,
        groups.len()
    ));
    for (index, group) in groups.iter().enumerate() {
        let names: Vec<&str> = group
            .iter()
            .map(|node| graph[*node].name.as_str())
            .collect();
        crate::ui::info(format!(
            "[dry-run] [{}] would run: {}",
            index + 1,
            names.join(", ")
        ));
    }
    Ok(())
}

pub fn dry_run_all() -> Result<()> {
    for pipeline in parser::read_pipelines()? {
        dry_run_pipeline(&pipeline.name)?;
    }
    Ok(())
}
