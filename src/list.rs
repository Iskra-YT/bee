use crate::parser::reader::config as reader;
use anyhow::Result;

pub fn list_pipelines() -> Result<()> {
    let pipelines = reader::read_pipelines_config()?;
    println!("Pipelines:");
    for pipeline in pipelines {
        println!("  {}", pipeline.name);
    }

    Ok(())
}

pub fn list_tasks() -> Result<()> {
    let tasks = reader::read_tasks_config()?;
    println!("Tasks:");
    for task in tasks {
        println!("  {}", task.name);
    }

    Ok(())
}

pub fn list_rules() -> Result<()> {
    let rules = reader::read_rules_config()?;
    println!("Rules:");
    for rule in rules {
        println!("  {}", rule.name);
    }

    Ok(())
}