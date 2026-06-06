use crate::parser::reader::config as reader;
use anyhow::Result;

pub fn list_pipelines() -> Result<()> {
    let pipelines = reader::read_pipelines_config()?;
    println!("[bee/info] Available pipelines:");
    for pipeline in pipelines {
        println!("[bee/info] \t{}", pipeline.name);
    }

    Ok(())
}

pub fn list_tasks() -> Result<()> {
    let tasks = reader::read_tasks_config()?;
    println!("[bee/info] Available tasks:");
    for task in tasks {
        println!("[bee/info] \t{}", task.name);
    }

    Ok(())
}

pub fn list_rules() -> Result<()> {
    let rules = reader::read_rules_config()?;
    println!("[bee/info] Available rules:");
    for rule in rules {
        println!("[bee/info] \t{}", rule.name);
    }

    Ok(())
}