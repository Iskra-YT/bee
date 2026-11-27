use crate::run::tasks;
use crate::{parser::Task, run::dag};
use std::thread;

pub fn find_parallel_groups(
    graph: &dag::DiGraph<Task, ()>,
    task_order: &[dag::NodeIndex],
) -> Vec<Vec<dag::NodeIndex>> {
    let mut groups: Vec<Vec<dag::NodeIndex>> = vec![];
    let mut remaining: Vec<dag::NodeIndex> = task_order.to_owned();

    while !remaining.is_empty() {
        let mut group = vec![];

        let completed: std::collections::HashSet<_> = groups
            .iter()
            .flatten()
            .chain(group.iter())
            .cloned()
            .collect();

        remaining.retain(|&node| {
            let dependencies: Vec<_> = graph
                .neighbors_directed(node, petgraph::Direction::Incoming)
                .collect();

            if dependencies.iter().all(|dep| completed.contains(dep)) {
                group.push(node);
                false
            } else {
                true
            }
        });

        if !group.is_empty() {
            groups.push(group);
        } else {
            groups.push(vec![remaining.remove(0)]);
        }
    }

    groups
}

/// Maximum number of tasks executed at the same time. Override with
/// `BEE_MAX_PARALLEL`; by default one task per available CPU is allowed.
fn concurrency_limit() -> usize {
    std::env::var("BEE_MAX_PARALLEL")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or_else(|| {
            thread::available_parallelism()
                .map(|count| count.get())
                .unwrap_or(4)
        })
}

pub fn run_parallel_tasks(
    graph: &dag::DiGraph<Task, ()>,
    group: Vec<dag::NodeIndex>,
) -> Vec<String> {
    let limit = concurrency_limit();
    let mut failed = vec![];

    // Tasks within one dependency layer are independent, so running them in
    // bounded chunks preserves semantics while capping resource usage.
    for chunk in group.chunks(limit) {
        failed.extend(run_chunk(graph, chunk));
    }

    failed
}

fn run_chunk(graph: &dag::DiGraph<Task, ()>, chunk: &[dag::NodeIndex]) -> Vec<String> {
    let mut handles = vec![];
    let mut failed = vec![];

    for &node in chunk {
        let task = graph[node].clone();
        let name = graph[node].name.clone();
        match thread::Builder::new()
            .name(format!("bee-{}", name))
            .spawn(move || -> Option<String> {
                tasks::run_task(task, false).err().map(|e| e.to_string())
            }) {
            Ok(handle) => handles.push((name, handle)),
            Err(err) => {
                crate::ui::einfo(format!(
                    "- {}: FAILED (could not start task thread: {})",
                    name, err
                ));
                failed.push(name);
            }
        }
    }

    for (name, handle) in handles {
        match handle.join() {
            Ok(Some(err)) => {
                crate::ui::einfo(format!(
                    "{} ({})",
                    crate::ui::red(&format!("- {}: FAILED", name)),
                    err
                ));
                failed.push(name);
            }
            Ok(None) => {}
            Err(_) => {
                crate::ui::einfo(format!(
                    "{} (thread crashed)",
                    crate::ui::red(&format!("- {}: PANICKED", name))
                ));
                failed.push(name);
            }
        }
    }

    failed
}
