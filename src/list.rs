use crate::parser::reader::config as reader;
use anyhow::Result;

pub fn list_pipelines() -> Result<()> {
    let pipelines = reader::read_pipelines_config()?;
    println!("Available pipelines:");
    for pipeline in pipelines {
        println!("\t{}", pipeline.name);
    }

    Ok(())
}

pub fn list_tasks() -> Result<()> {
    let tasks = reader::read_tasks_config()?;
    println!("Available tasks:");
    for task in tasks {
        println!("\t{}", task.name);
    }

    Ok(())
}

pub fn list_rules() -> Result<()> {
    let rules = reader::read_rules_config()?;
    println!("Available rules:");
    for rule in rules {
        println!("\t{}", rule.name);
    }

    Ok(())
}