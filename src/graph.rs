use std::collections::HashMap;
use crate::parser;
use crate::parser::reader::config as reader;
use crate::file;
use crate::run::dag;

pub enum GraphFormat {
    Tree,
    Dot,
    Mermaid,
}

pub fn render_pipeline_graph(name: &str, format: GraphFormat) -> anyhow::Result<String> {
    let content = file::get_file_content(&format!("bee/pipelines/{}.yml", name))?;
    let pipeline = reader::read_pipeline_from_string(&content)?;
    let (topo_order, graph) = dag::build_dag(pipeline)?;

    let labels: HashMap<_, _> = topo_order.iter().enumerate().map(|(_, &node)| {
        (node, graph[node].name.clone())
    }).collect();

    match format {
        GraphFormat::Tree => Ok(render_tree(&graph, &topo_order, &labels)),
        GraphFormat::Dot => Ok(render_dot(name, &graph, &topo_order, &labels)),
        GraphFormat::Mermaid => Ok(render_mermaid(name, &graph, &topo_order, &labels)),
    }
}

pub fn render_all_pipelines(format: GraphFormat) -> anyhow::Result<String> {
    let pipelines = reader::read_pipelines()?;
    let mut output = String::new();

    for pipeline in &pipelines {
        let content = file::get_file_content(&format!("bee/pipelines/{}.yml", pipeline.name))?;
        let pipe = reader::read_pipeline_from_string(&content)?;
        let (topo_order, graph) = dag::build_dag(pipe)?;

        let labels: HashMap<_, _> = topo_order.iter().enumerate().map(|(_, &node)| {
            (node, graph[node].name.clone())
        }).collect();

        match format {
            GraphFormat::Tree => {
                output.push_str(&format!("Pipeline: {}\n", pipeline.name));
                output.push_str(&render_tree(&graph, &topo_order, &labels));
                output.push('\n');
            }
            GraphFormat::Dot => {
                output.push_str(&render_dot(&pipeline.name, &graph, &topo_order, &labels));
                output.push('\n');
            }
            GraphFormat::Mermaid => {
                output.push_str(&render_mermaid(&pipeline.name, &graph, &topo_order, &labels));
                output.push('\n');
            }
        }
    }

    Ok(output)
}

fn render_tree(
    graph: &dag::DiGraph<parser::Task, ()>,
    order: &Vec<dag::NodeIndex>,
    labels: &HashMap<dag::NodeIndex, String>,
) -> String {
    let mut output = String::new();

    for &node in order {
        let deps: Vec<String> = graph
            .neighbors_directed(node, petgraph::Direction::Incoming)
            .map(|n| labels.get(&n).cloned().unwrap_or_default())
            .collect();

        let task_name = labels.get(&node).cloned().unwrap_or_default();

        if deps.is_empty() {
            output.push_str(&format!("  ├── {}\n", task_name));
        } else {
            output.push_str(&format!("  ├── {} (depends on: {})\n", task_name, deps.join(", ")));
        }
    }

    output
}

fn render_dot(
    pipeline_name: &str,
    graph: &dag::DiGraph<parser::Task, ()>,
    order: &Vec<dag::NodeIndex>,
    labels: &HashMap<dag::NodeIndex, String>,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("digraph {} {{\n", pipeline_name));
    output.push_str("  rankdir=LR;\n");
    output.push_str("  node [shape=box, style=rounded];\n\n");

    let node_ids: HashMap<_, _> = order.iter().enumerate().map(|(i, &node)| {
        (node, format!("n{}", i))
    }).collect();

    for (&node, id) in &node_ids {
        let name = labels.get(&node).cloned().unwrap_or_default();
        output.push_str(&format!("  {} [label=\"{}\"];\n", id, name));
    }

    output.push('\n');

    for &node in order {
        for dep in graph.neighbors_directed(node, petgraph::Direction::Incoming) {
            if let (Some(from_id), Some(to_id)) = (node_ids.get(&dep), node_ids.get(&node)) {
                output.push_str(&format!("  {} -> {};\n", from_id, to_id));
            }
        }
    }

    output.push_str("}\n");
    output
}

fn render_mermaid(
    _pipeline_name: &str,
    graph: &dag::DiGraph<parser::Task, ()>,
    order: &Vec<dag::NodeIndex>,
    labels: &HashMap<dag::NodeIndex, String>,
) -> String {
    let mut output = String::new();
    output.push_str("graph LR;\n");

    for &node in order {
        for dep in graph.neighbors_directed(node, petgraph::Direction::Incoming) {
            let from = labels.get(&dep).cloned().unwrap_or_default();
            let to = labels.get(&node).cloned().unwrap_or_default();
            output.push_str(&format!("  {}-->{};\n", from, to));
        }
    }

    output
}
