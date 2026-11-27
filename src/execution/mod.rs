pub mod context;
pub mod error;
pub mod executor;
pub mod platform;
pub mod process;
pub mod registry;
pub mod resolution;
pub mod script;
pub mod shell;

pub use context::ExecutionContext;
pub use error::ExecutionError;
pub use executor::{ExecutionResult, Executor};
pub use registry::{execute_task, ExecutorRegistry};
pub use resolution::{program_exists, resolve_interpreter, resolve_interpreter_for_task};
