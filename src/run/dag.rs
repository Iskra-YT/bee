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
        let path = crate::safety::safe_path("bee/tasks", "task", task_name)?
            .with_extension("yml");
        let content = file::get_file_content(&path.to_string_lossy())
            .map_err(|e| anyhow::anyhow!("failed to read task '{}': {}", task_name, e))?;
        let task = reader::read_task_from_string(&content, task_name)
            .map_err(|e| anyhow::anyhow!("invalid task '{}': {}", task_name, e))?;
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
                    return Err(anyhow::anyhow!(
                        "task '{}' depends on unknown task '{}' (add it to the pipeline or fix depends_on)",
                        task.name,
                        dep
                    ));
                }
            }
        }
    }

    let topo_order = toposort(&graph, None).map_err(|cycle| {
        let node = cycle.node_id();
        let name = graph
            .node_weight(node)
            .map(|task| task.name.clone())
            .unwrap_or_else(|| format!("node {:?}", node));
        anyhow::anyhow!(
            "cycle detected in task dependencies at task '{}'",
            name
        )
    })?;

    Ok((topo_order, graph))
}
