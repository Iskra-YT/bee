use crate::{parser::Task, run::dag};

pub fn find_parallel_groups(
    graph: &dag::DiGraph<Task, ()>,
    task_order: &Vec<dag::NodeIndex>,
) -> Vec<Vec<dag::NodeIndex>> {
    let mut groups: Vec<Vec<dag::NodeIndex>> = vec![];
    let mut remaining: Vec<dag::NodeIndex> = task_order.clone();

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
