pub mod reader;

use serde::Serialize;

#[derive(Serialize)]
pub struct Task {
    pub name: String,
    pub run: String,
    pub depends_on: Option<Vec<String>>
}

impl Task {
    pub fn clone (&self) -> Task {
        Task {
            name: self.name.clone(),
            run: self.run.clone(),
            depends_on: self.depends_on.clone()
        }
    }
}

#[derive(Serialize)]
pub struct Pipeline {
    pub name: String,
    pub tasks: Vec<String>
}

#[derive(Serialize)]
pub struct RuleAction {
    pub input: String,
    pub output: String
}

#[derive(Serialize)]
pub struct Rule {
    pub task: String,
    pub actions: Vec<RuleAction>
}

#[derive(Serialize)]
pub struct MainConfig {
    pub tasks: Vec<String>,
    pub rules: Vec<String>,
    pub pipelines: Vec<String>
}

pub struct PipelineConfig {
    pub name: String
}

pub struct TaskConfig {
    pub name: String
}

pub struct RuleConfig  {
    pub name: String
}