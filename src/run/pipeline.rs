use crate::run::dag;
use anyhow::Result;
use crate::run::parallel;

pub fn run_pipeline(config: crate::parser::Pipeline) -> Result<()> {
    println!("[bee/info] Running pipeline: {}", config.name);
    let (task_order, graph) = dag::build_dag(config.clone())?;

    let order = parallel::find_parallel_groups(&graph, &task_order);
    
    for layer in order.iter() {
        parallel::run_parallel_tasks(&graph, layer.clone())?;
    }
    Ok(())
}