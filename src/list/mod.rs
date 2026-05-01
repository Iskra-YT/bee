use crate::parser::reader;

pub fn list_pipelines() {
    let pipelines = reader::read_pipelines();
    println!("Available pipelines:");
    for pipeline in pipelines {
        println!("\t{}", pipeline.name);
    }
}

pub fn list_tasks() {
    let tasks = reader::read_tasks();
    println!("Available tasks:");
    for task in tasks {
        println!("\t{}", task.name);
    }
}

pub fn list_rules() {
    let rules = reader::read_rules();
    println!("Available rules:");
    for rule in rules {
        println!("\t{}", rule.name);
    }
}