mod pipeline;
pub mod dag;
mod parallel;
pub mod tasks;
pub mod cache;

use anyhow::Result;
use crate::parser::reader::config as parser;

pub fn run_pipeline(name: String, config: Option<crate::parser::Pipeline>) -> Result<()> {
    if let Some(config) = config {
        pipeline::run_pipeline(config)?;
    } else {
        let pipelines = parser::read_pipelines()?;
        if let Some(pipeline) = pipelines.into_iter().find(|p| p.name == name) {
            pipeline::run_pipeline(pipeline)?;
        } else {
            eprintln!("Error: pipeline '{}' not found", name);
        }
    }

    Ok(())
}

pub fn run_all() -> Result<()>{
    let pipelines = parser::read_pipelines().unwrap_or_else(|e| {
        eprint!("Error: {}", e);
        vec![]
    });

    for pipeline in pipelines {
        run_pipeline(pipeline.name.clone(), Some(pipeline))?;
    }

    Ok(())
}

pub fn run_task(name: String) -> Result<()> {
    let task_file = format!("bee/tasks/{}.yml", name);
    let content = crate::file::get_file_content(&task_file)?;
    let task = parser::read_task_from_string(&content, &name)?;
    
    tasks::run_task(task)?;
    Ok(())
}