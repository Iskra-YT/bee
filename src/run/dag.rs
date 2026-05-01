use anyhow::Result;
use crate::file;
use crate::parser::Task;
use crate::parser::reader::config as reader;
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use petgraph::prelude::NodeIndex;

pub fn build_dag(config: crate::parser::Pipeline) -> Result<(Vec<NodeIndex>, DiGraph<Task, ()>)> {
    let mut graph = DiGraph::<Task, ()>::new();
    let mut tasks: Vec<Task> = Vec::new();

    config.tasks.iter().for_each(|task_name| {
        tasks.push(
            reader::read_task_from_string(
                &file::get_file_content(&format!("bee/tasks/{}.yml", task_name))
                    .unwrap_or_default(),
                task_name,
            )
            .unwrap_or_else(|e| panic!("Failed to read task {}: {}", task_name, e)),
        );
    });

    let node_indices = tasks
        .iter()
        .map(|task| graph.add_node(task.clone()))
        .collect::<Vec<_>>();

    for (i, task) in tasks.iter().enumerate() {
        if let Some(deps) = &task.depends_on {
            for dep in deps {
                if let Some(dep_index) = tasks.iter().position(|t| &t.name == dep) {
                    graph.add_edge(node_indices[dep_index], node_indices[i], ());
                } else {
                    eprintln!(
                        "Warning: Task '{}' depends on unknown task '{}'",
                        task.name, dep
                    );
                }
            }
        }
    }

    let topo_order = toposort(&graph, None)
        .map_err(|cycle| anyhow::anyhow!("Cycle detected in task dependencies: {:?}", cycle.node_id()))?;

    Ok((topo_order, graph))
}
