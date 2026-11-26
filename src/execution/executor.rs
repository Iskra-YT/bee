use crate::execution::context::ExecutionContext;
use crate::execution::error::ExecutionError;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

pub trait Executor {
    fn execute(&self, context: &ExecutionContext) -> Result<ExecutionResult, ExecutionError>;
}
