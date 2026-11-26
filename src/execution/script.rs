use crate::execution::context::ExecutionContext;
use crate::execution::error::ExecutionError;
use crate::execution::executor::{ExecutionResult, Executor};
use crate::execution::process::run_process;
use crate::execution::shell::program_exists;

pub struct ScriptExecutor;

impl Executor for ScriptExecutor {
    fn execute(&self, context: &ExecutionContext) -> Result<ExecutionResult, ExecutionError> {
        let interpreter =
            context
                .interpreter
                .as_deref()
                .ok_or_else(|| ExecutionError::MissingField {
                    task: context.task_name.clone(),
                    field: "interpreter".to_string(),
                })?;
        let script = context
            .script
            .as_deref()
            .ok_or_else(|| ExecutionError::MissingField {
                task: context.task_name.clone(),
                field: "script".to_string(),
            })?;

        if !program_exists(interpreter) {
            return Err(ExecutionError::InterpreterNotFound {
                task: context.task_name.clone(),
                interpreter: interpreter.to_string(),
            });
        }

        let mut args = Vec::with_capacity(context.args.len() + 1);
        args.push(script.to_string());
        args.extend(context.args.iter().cloned());
        run_process(context, interpreter, &args)
    }
}
