pub mod reader;

use serde::{Serialize, Deserialize};

#[derive(Serialize)]
pub struct Task {
    pub name: String,
    pub run: String,
    #[serde(skip_serializing_if = "Option::is_none")]
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

impl Pipeline {
    pub fn clone (&self) -> Pipeline {
        Pipeline {
            name: self.name.clone(),
            tasks: self.tasks.clone()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rule {
    pub name: String,
    pub task: String,
    #[serde(default)]
    pub triggers: Vec<RuleTrigger>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum RuleTrigger {
    #[serde(rename = "file_change")]
    FileChange { paths: Vec<String> },
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "schedule")]
    Schedule { cron: String },
    #[serde(rename = "task_completed")]
    TaskCompleted { task: String },
    #[serde(rename = "task_failed")]
    TaskFailed { task: String },
    #[serde(rename = "dependency_changed")]
    DependencyChanged { task: String },
    #[serde(rename = "git_changed")]
    GitChanged { paths: Vec<String> },
    #[serde(rename = "git_tag")]
    GitTag { pattern: Option<String> },
    #[serde(rename = "env_changed")]
    EnvChanged { vars: Vec<String> },
    #[serde(rename = "dependency_missing")]
    DependencyMissing { path: String },
    #[serde(rename = "checksum_changed")]
    ChecksumChanged { paths: Vec<String> },
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dependency {
    pub name: String,
    #[serde(rename = "type")]
    pub dep_type: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DepsConfig {
    pub dependencies: Vec<Dependency>,
}