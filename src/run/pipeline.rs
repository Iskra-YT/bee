use crate::run::dag;
use anyhow::Result;

pub fn run_pipeline(config: crate::parser::Pipeline) -> Result<()> {
    println!("Running pipeline: {}", config.name);
    let (task_order, graph) = dag::build_dag(config)?;

    for (i, node) in task_order.iter().enumerate() {
        let task = &graph[*node];
        println!("Running task {}: {}", i + 1, task.name);
        println!("Command: {}", task.run);
    }
    Ok(())
}