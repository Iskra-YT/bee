use crate::parser::reader::config as reader;
use crate::run::dag;
use crate::run::parallel;
use anyhow::Result;
use std::collections::HashSet;

pub fn run_pipeline(config: crate::parser::Pipeline) -> Result<()> {
    let mut seen: Vec<String> = vec![];
    run_pipeline_recursive(config, &mut seen, 0)
}

fn run_pipeline_recursive(
    config: crate::parser::Pipeline,
    seen: &mut Vec<String>,
    depth: usize,
) -> Result<()> {
    if seen.contains(&config.name) {
        let mut cycle: Vec<String> = seen.clone();
        cycle.push(config.name.clone());
        return Err(anyhow::anyhow!(
            "Subpipeline cycle detected: {}",
            cycle.join(" -> ")
        ));
    }
    seen.push(config.name.clone());

    let mut sub_failed: Vec<String> = vec![];

    for sub_name in &config.pipelines {
        match reader::read_pipeline(sub_name) {
            Ok(sub) => {
                println!(
                    "  {}Pipeline '{}' invokes subpipeline '{}'",
                    "  ".repeat(depth),
                    config.name,
                    sub.name
                );
                if let Err(e) = run_pipeline_recursive(sub, seen, depth + 1) {
                    sub_failed.push(format!("{}: {}", sub_name, e));
                }
            }
            Err(e) => {
                sub_failed.push(format!("{}: {}", sub_name, e));
            }
        }
    }

    seen.pop();

    if !sub_failed.is_empty() {
        eprintln!(
            "\x1b[31mPipeline '{}' failed:\x1b[0m subpipeline(s) failed: {}",
            config.name,
            sub_failed.join(", ")
        );
        return Err(anyhow::anyhow!(
            "Pipeline '{}' failed: subpipeline(s) failed",
            config.name
        ));
    }

    run_pipeline_tasks(config)
}

fn run_pipeline_tasks(config: crate::parser::Pipeline) -> Result<()> {
    let (task_order, graph) = dag::build_dag(config.clone())?;
    let order = parallel::find_parallel_groups(&graph, &task_order);

    let total: usize = order.iter().map(|g| g.len()).sum();
    println!(
        "Pipeline: {} ({} tasks, {} group(s))",
        config.name,
        total,
        order.len()
    );

    let mut failed: HashSet<String> = HashSet::new();
    let mut skipped: Vec<String> = vec![];

    for (i, layer) in order.iter().enumerate() {
        let filtered: Vec<dag::NodeIndex> = layer
            .iter()
            .copied()
            .filter(|&node| {
                let deps: Vec<String> = graph
                    .neighbors_directed(node, petgraph::Direction::Incoming)
                    .map(|n| graph[n].name.clone())
                    .collect();
                !deps.iter().any(|d| failed.contains(d))
            })
            .collect();

        let skipped_in_layer: Vec<String> = layer
            .iter()
            .filter(|&&node| !filtered.contains(&node))
            .map(|&n| graph[n].name.clone())
            .collect();

        if !skipped_in_layer.is_empty() {
            for s in &skipped_in_layer {
                println!(
                    "  [{}/{}] \x1b[33m{}: SKIPPED\x1b[0m (dependency failed)",
                    i + 1,
                    order.len(),
                    s
                );
            }
            skipped.extend(skipped_in_layer);
        }

        if filtered.is_empty() {
            continue;
        }

        let names: Vec<String> = filtered.iter().map(|&n| graph[n].name.clone()).collect();
        if filtered.len() > 1 {
            println!(
                "  [{}/{}] {} (parallel)",
                i + 1,
                order.len(),
                names.join(", ")
            );
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
        eprintln!(
            "\x1b[31mPipeline '{}' failed:\x1b[0m {}",
            config.name,
            fail_list.join(", ")
        );
        if !skipped.is_empty() {
            eprintln!("\x1b[33mSkipped:\x1b[0m {}", skipped.join(", "));
        }
        Err(anyhow::anyhow!(
            "Pipeline '{}' failed: {}",
            config.name,
            fail_list.join(", ")
        ))
    }
}
