use crate::run::dag;
use anyhow::Result;
use crate::run::parallel;
use std::collections::HashSet;

pub fn run_pipeline(config: crate::parser::Pipeline) -> Result<()> {
    let (task_order, graph) = dag::build_dag(config.clone())?;
    let order = parallel::find_parallel_groups(&graph, &task_order);

    let total: usize = order.iter().map(|g| g.len()).sum();
    println!("Pipeline: {} ({} tasks, {} group(s))", config.name, total, order.len());

    let mut failed: HashSet<String> = HashSet::new();
    let mut skipped: Vec<String> = vec![];

    for (i, layer) in order.iter().enumerate() {
        let filtered: Vec<dag::NodeIndex> = layer.iter().copied().filter(|&node| {
            let deps: Vec<String> = graph
                .neighbors_directed(node, petgraph::Direction::Incoming)
                .map(|n| graph[n].name.clone())
                .collect();
            !deps.iter().any(|d| failed.contains(d))
        }).collect();

        let skipped_in_layer: Vec<String> = layer.iter().filter(|&&node| {
            !filtered.contains(&node)
        }).map(|&n| graph[n].name.clone()).collect();

        if !skipped_in_layer.is_empty() {
            for s in &skipped_in_layer {
                println!("  [{}/{}] \x1b[33m{}: SKIPPED\x1b[0m (dependency failed)", i + 1, order.len(), s);
            }
            skipped.extend(skipped_in_layer);
        }

        if filtered.is_empty() {
            continue;
        }

        let names: Vec<String> = filtered.iter().map(|&n| graph[n].name.clone()).collect();
        if filtered.len() > 1 {
            println!("  [{}/{}] {} (parallel)", i + 1, order.len(), names.join(", "));
        } else {
            println!("  [{}/{}] {}", i + 1, order.len(), names.join(", "));
        }

        let layer_failed = parallel::run_parallel_tasks(&graph, filtered);
        failed.extend(layer_failed);
    }

    println!();
    if failed.is_empty() {
        println!("Pipeline '{}' completed successfully", config.name);
        Ok(())
    } else {
        let mut fail_list: Vec<String> = failed.into_iter().collect();
        fail_list.sort();
        eprintln!("\x1b[31mPipeline '{}' failed:\x1b[0m {}", config.name, fail_list.join(", "));
        if !skipped.is_empty() {
            eprintln!("\x1b[33mSkipped:\x1b[0m {}", skipped.join(", "));
        }
        Err(anyhow::anyhow!("Pipeline '{}' failed: {}", config.name, fail_list.join(", ")))
    }
}
