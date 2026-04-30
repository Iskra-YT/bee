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