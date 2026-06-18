use crate::run::dag;
use anyhow::Result;
use crate::run::parallel;

pub fn run_pipeline(config: crate::parser::Pipeline) -> Result<()> {
    let (task_order, graph) = dag::build_dag(config.clone())?;
    let order = parallel::find_parallel_groups(&graph, &task_order);

    let total: usize = order.iter().map(|g| g.len()).sum();
    println!("Pipeline: {} ({} tasks, {} group(s))", config.name, total, order.len());

    for (i, layer) in order.iter().enumerate() {
        let names: Vec<String> = layer.iter().map(|&n| graph[n].name.clone()).collect();
        if layer.len() > 1 {
            println!("  [{}/{}] {} (parallel)", i + 1, order.len(), names.join(", "));
        } else {
            println!("  [{}/{}] {}", i + 1, order.len(), names.join(", "));
        }
        parallel::run_parallel_tasks(&graph, layer.clone())?;
    }
    Ok(())
}