use crate::execution::context::ExecutionContext;
use crate::execution::error::ExecutionError;
use crate::execution::executor::{ExecutionResult, Executor};
use crate::execution::process::run_process;
use crate::execution::resolution::resolve_interpreter;

pub struct ScriptExecutor;

impl Executor for ScriptExecutor {
    fn execute(&self, context: &ExecutionContext) -> Result<ExecutionResult, ExecutionError> {
        let candidates = if context.interpreter_candidates.is_empty() {
            context
                .interpreter
                .as_deref()
                .map(|interpreter| vec![interpreter.to_string()])
                .unwrap_or_default()
        } else {
            context.interpreter_candidates.clone()
        };

        if candidates.is_empty() {
            return Err(ExecutionError::MissingField {
                task: context.task_name.clone(),
                field: "interpreter".to_string(),
            });
        }

        // Discovery is a one-time decision. Once an interpreter is selected,
        // its exit status is final and no other candidate is launched.
        let interpreter = resolve_interpreter(&context.task_name, &candidates)?;

        let script = context
            .script
            .as_deref()
            .ok_or_else(|| ExecutionError::MissingField {
                task: context.task_name.clone(),
                field: "script".to_string(),
            })?;

        let mut args = Vec::with_capacity(context.args.len() + 1);
        args.push(script.to_string());
        args.extend(context.args.iter().cloned());
        run_process(context, &interpreter, &args)
    }
}
