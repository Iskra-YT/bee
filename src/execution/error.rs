use std::fmt;

#[derive(Debug)]
pub enum ExecutionError {
    MissingField {
        task: String,
        field: String,
    },
    InvalidConfig {
        task: String,
        message: String,
    },
    UnsupportedExecutor {
        task: String,
        executor: String,
    },
    ExecutorNotFound {
        task: String,
        candidates: Vec<String>,
    },
    UnsupportedPlatform {
        task: String,
        platform: String,
    },
    ProgramNotFound {
        task: String,
        program: String,
    },
    InterpreterNotFound {
        task: String,
        interpreter: String,
    },
    ShellNotFound {
        task: String,
        shell: String,
    },
    WorkingDirectory {
        task: String,
        path: String,
        source: std::io::Error,
    },
    Spawn {
        task: String,
        program: String,
        source: std::io::Error,
    },
    Io {
        task: String,
        message: String,
        source: std::io::Error,
    },
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionError::MissingField { task, field } => {
                write!(f, "Task '{}': missing required field '{}'", task, field)
            }
            ExecutionError::InvalidConfig { task, message } => {
                write!(f, "Task '{}': {}", task, message)
            }
            ExecutionError::UnsupportedExecutor { task, executor } => {
                write!(f, "Task '{}': unsupported executor '{}'", task, executor)
            }
            ExecutionError::ExecutorNotFound { task, candidates } => {
                write!(
                    f,
                    "Task '{}': no available executor found; tried: {} (install one of these or update the 'executor' list)",
                    task,
                    candidates.join(", ")
                )
            }
            ExecutionError::UnsupportedPlatform { task, platform } => {
                write!(f, "Task '{}': unsupported platform '{}'", task, platform)
            }
            ExecutionError::ProgramNotFound { task, program } => {
                write!(f, "Task '{}': program '{}' not found", task, program)
            }
            ExecutionError::InterpreterNotFound { task, interpreter } => {
                write!(
                    f,
                    "Task '{}': interpreter '{}' not found",
                    task, interpreter
                )
            }
            ExecutionError::ShellNotFound { task, shell } => {
                write!(f, "Task '{}': shell '{}' not found", task, shell)
            }
            ExecutionError::WorkingDirectory { task, path, source } => {
                write!(
                    f,
                    "Task '{}': invalid working directory '{}': {}",
                    task, path, source
                )
            }
            ExecutionError::Spawn {
                task,
                program,
                source,
            } => {
                write!(
                    f,
                    "Task '{}': failed to launch '{}': {}",
                    task, program, source
                )
            }
            ExecutionError::Io {
                task,
                message,
                source,
            } => {
                write!(f, "Task '{}': {}: {}", task, message, source)
            }
        }
    }
}

impl std::error::Error for ExecutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecutionError::WorkingDirectory { source, .. }
            | ExecutionError::Spawn { source, .. }
            | ExecutionError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
