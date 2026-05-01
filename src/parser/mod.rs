pub mod reader;

use serde::Serialize;

#[derive(Serialize)]
pub struct TaskConfig {
    pub run: String,
    pub depends_on: Option<Vec<String>>
}

#[derive(Serialize)]
pub struct MainConfig {
    pub tasks: Vec<String>,
    pub rules: Vec<String>,
    pub pipelines: Vec<String>
}

pub struct Pipeline {
    pub name: String
}

pub struct Task {
    pub name: String
}

pub struct Rule {
    pub name: String
}