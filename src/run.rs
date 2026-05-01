mod pipeline;

use crate::parser::reader::config as parser;

pub fn run_pipeline(name: String, config: Option<crate::parser::Pipeline>) {
    if let Some(config) = config {
        pipeline::run_pipeline(name, config);
    } else {
        eprintln!("Error: Pipeline '{}' not found", name);
    }
}

pub fn run_all() {
    let pipelines = parser::read_pipelines().unwrap_or_else(|e| {
        eprint!("Error reading pipelines: {}", e);
        vec![]
    });

    for pipeline in pipelines {
        run_pipeline(pipeline.name.clone(), Some(pipeline));
    }
}

pub fn run_task(name: String) {
    println!("Running task: {}", name);
}