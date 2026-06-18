use crate::run::dag;
use anyhow::Result;
use crate::run::parallel;

pub fn run_pipeline(config: crate::parser::Pipeline) -> Result<()> {
    println!("[bee/info] Pipeline: {}", config.name);
    let (task_order, graph) = dag::build_dag(config.clone())?;

    let order = parallel::find_parallel_groups(&graph, &task_order);

    println!("[bee/info] Execution plan ({} group(s)):", order.len());
    for (i, group) in order.iter().enumerate() {
        let names: Vec<String> = group.iter().map(|&n| graph[n].name.clone()).collect();
        if group.len() > 1 {
            println!("[bee/info]   Group {} (parallel): {}", i + 1, names.join(", "));
        } else {
            println!("[bee/info]   Group {}: {}", i + 1, names.join(", "));
        }
    }
    println!();

    for (i, layer) in order.iter().enumerate() {
        if layer.len() > 1 {
            println!("[bee/info] Executing group {} ({} tasks in parallel)...", i + 1, layer.len());
        } else {
            println!("[bee/info] Executing group {}...", i + 1);
        }
        parallel::run_parallel_tasks(&graph, layer.clone())?;
    }
    println!("[bee/info] Pipeline '{}' complete", config.name);
    Ok(())
}