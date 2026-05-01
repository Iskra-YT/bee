mod pipeline;
pub mod dag;
mod parallel;

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
            eprintln!("Pipeline '{}' not found", name);
        }
    }

    Ok(())
}

pub fn run_all() -> Result<()>{
    let pipelines = parser::read_pipelines().unwrap_or_else(|e| {
        eprint!("Error reading pipelines: {}", e);
        vec![]
    });

    for pipeline in pipelines {
        run_pipeline(pipeline.name.clone(), Some(pipeline))?;
    }

    Ok(())
}

pub fn run_task(name: String) -> Result<()> {
    println!("Running task: {}", name);
    Ok(())
}