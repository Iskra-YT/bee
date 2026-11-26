use crate::file;
use crate::parser::reader::config as reader;
use crate::parser::Task;
use anyhow::Result;
use petgraph::algo::toposort;
pub use petgraph::graph::DiGraph;
pub use petgraph::prelude::NodeIndex;

pub fn build_dag(config: crate::parser::Pipeline) -> Result<(Vec<NodeIndex>, DiGraph<Task, ()>)> {
    let mut graph = DiGraph::<Task, ()>::new();
    let mut tasks: Vec<Task> = Vec::new();

    for task_name in &config.tasks {
        let content = file::get_file_content(&format!("bee/tasks/{}.yml", task_name))
            .map_err(|e| anyhow::anyhow!("Failed to read task '{}': {}", task_name, e))?;
        let task = reader::read_task_from_string(&content, task_name)
            .map_err(|e| anyhow::anyhow!("Failed to read task '{}': {}", task_name, e))?;
        tasks.push(task);
    }

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
                        "Warning: task '{}' depends on unknown task '{}'",
                        task.name, dep
                    );
                }
            }
        }
    }

    let topo_order = toposort(&graph, None).map_err(|cycle| {
        anyhow::anyhow!("Cycle detected in task dependencies: {:?}", cycle.node_id())
    })?;

    Ok((topo_order, graph))
}
