use crate::run::dag;
use anyhow::Result;
use crate::run::parallel;

pub fn run_pipeline(config: crate::parser::Pipeline) -> Result<()> {
    println!("Running pipeline: {}", config.name);
    let (task_order, graph) = dag::build_dag(config.clone())?;

    let order = parallel::find_parallel_groups(&graph, &task_order);
    
    for (i, layer) in order.iter().enumerate() {
        for (j, index) in layer.iter().enumerate() {
            let task = &graph[*index];
            println!("{i}:   {j} - {}: {} ", task.name, task.run)
        }
    }
    Ok(())
}