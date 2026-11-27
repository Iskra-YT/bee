use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct ExecutionOptions {
    pub timeout: Option<Duration>,
    /// When true and the process is attached to a terminal, stdout/stderr are
    /// inherited so interactive programs (ssh, sudo) work naturally. Output is
    /// not captured for logs in this mode.
    pub interactive_tty: bool,
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub task_name: String,
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub inherit_env: bool,
    pub options: ExecutionOptions,
    pub shell: Option<String>,
    pub shell_command: Option<String>,
    pub interpreter: Option<String>,
    /// Ordered interpreter candidates; takes precedence over `interpreter`.
    pub interpreter_candidates: Vec<String>,
    pub script: Option<String>,
}

impl ExecutionContext {
    pub fn new(task_name: impl Into<String>, program: impl Into<String>) -> Self {
        Self {
            task_name: task_name.into(),
            program: program.into(),
            args: Vec::new(),
            working_dir: None,
            env: BTreeMap::new(),
            inherit_env: true,
            options: ExecutionOptions::default(),
            shell: None,
            shell_command: None,
            interpreter: None,
            interpreter_candidates: Vec::new(),
            script: None,
        }
    }
}
