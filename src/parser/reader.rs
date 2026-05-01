use crate::parser::{Pipeline, Task, Rule};

pub fn read_pipelines() -> Vec<Pipeline> {
    vec![
        Pipeline { name: "Pipeline 1".to_string() },
        Pipeline { name: "Pipeline 2".to_string() },
    ]
}

pub fn read_tasks() -> Vec<Task> {
    vec![
        Task { name: "Task 1".to_string() },
        Task { name: "Task 2".to_string() },
    ]
}

pub fn read_rules() -> Vec<Rule> {
    vec![
        Rule { name: "Rule 1".to_string() },
        Rule { name: "Rule 2".to_string() },
    ]
}