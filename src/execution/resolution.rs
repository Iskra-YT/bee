//! Executable discovery and executor fallback resolution.
//!
//! Bee's `executor` field can name a built-in execution mode (`process`,
//! `shell`, `script`, `legacy`) or an executable (possibly a list of
//! alternatives). This module contains the platform-aware discovery logic
//! used by both the executor registry and the shell/script executors so that
//! there is a single implementation of "is this program available?".

use crate::execution::error::ExecutionError;
use std::env;
use std::path::Path;

/// Extensions used by Windows when an executable is invoked without one.
pub const DEFAULT_WINDOWS_PATHEXT: &str = ".EXE;.CMD;.BAT;.COM";

/// Return the executable candidates configured for a task, in priority order.
///
/// A single string that is not a built-in execution mode is treated as a
/// one-element fallback list. Built-in modes return an empty candidate list
/// because they are resolved by the executor registry, not by PATH lookup.
pub fn executor_candidates(task: &crate::parser::Task) -> Vec<String> {
    task.executor_candidates()
}

/// Select the first configured executable that is available on this system.
///
/// Candidates are checked strictly in the order supplied. This function only
/// performs discovery: once a candidate has been selected and launched, its
/// exit status is final and no further candidate is tried.
pub fn resolve_executor(task: &str, candidates: &[String]) -> Result<String, ExecutionError> {
    if candidates.is_empty() {
        return Err(ExecutionError::InvalidConfig {
            task: task.to_string(),
            message: "'executor' list cannot be empty; specify at least one executable".to_string(),
        });
    }

    for candidate in candidates {
        let candidate = candidate.trim();
        if candidate.is_empty() {
            continue;
        }
        if program_exists(candidate) {
            return Ok(candidate.to_string());
        }
    }

    Err(ExecutionError::ExecutorNotFound {
        task: task.to_string(),
        candidates: candidates.to_vec(),
    })
}

/// Resolve the first available executor configured on `task`.
pub fn resolve_executor_for_task(task: &crate::parser::Task) -> Result<String, ExecutionError> {
    let candidates = executor_candidates(task);
    resolve_executor(&task.name, &candidates)
}

/// Check whether an executable is available as a path or on `PATH`.
///
/// On Windows (or when `PATHEXT` is explicitly set) the usual executable
/// extensions are tried in addition to the literal name, so `python` matches
/// `python.exe`, `python.cmd`, etc. On Unix-like systems the literal name is
/// used, matching the behavior of `std::process::Command`.
pub fn program_exists(program: &str) -> bool {
    let program = program.trim();
    if program.is_empty() {
        return false;
    }

    let extensions = path_extensions();
    let names = invocation_names(program, &extensions);

    if is_path_like(program) {
        return names.iter().any(|name| Path::new(name).is_file());
    }

    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };

    for dir in env::split_paths(&path_var) {
        if names.iter().any(|name| dir.join(name).is_file()) {
            return true;
        }
    }
    false
}

/// Names to probe for a candidate, including Windows executable extensions.
fn invocation_names(program: &str, extensions: &[String]) -> Vec<String> {
    let mut names = vec![program.to_string()];
    if extensions.is_empty() || has_known_extension(program, extensions) {
        return names;
    }
    for extension in extensions {
        names.push(format!("{}{}", program, extension));
    }
    names
}

/// Parse `PATHEXT` into normalized, lowercase extensions.
fn parse_path_extensions(raw: &str) -> Vec<String> {
    let mut extensions = Vec::new();
    for entry in raw.split(';').map(str::trim).filter(|e| !e.is_empty()) {
        let extension = if entry.starts_with('.') {
            entry.to_ascii_lowercase()
        } else {
            format!(".{}", entry.to_ascii_lowercase())
        };
        if !extensions.contains(&extension) {
            extensions.push(extension);
        }
    }
    extensions
}

/// Effective executable extensions for the current environment.
fn path_extensions() -> Vec<String> {
    match env::var("PATHEXT") {
        Ok(raw) if !raw.trim().is_empty() => parse_path_extensions(&raw),
        _ if cfg!(windows) => parse_path_extensions(DEFAULT_WINDOWS_PATHEXT),
        _ => Vec::new(),
    }
}

fn has_known_extension(program: &str, extensions: &[String]) -> bool {
    let lower = program.to_ascii_lowercase();
    extensions
        .iter()
        .any(|extension| lower.ends_with(extension))
}

/// A candidate containing a path separator is checked as a path, not via PATH.
fn is_path_like(program: &str) -> bool {
    program.contains('/') || program.contains('\\') || Path::new(program).components().count() > 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_windows_path_extensions() {
        let extensions = parse_path_extensions(".EXE;.CMD;.BAT");
        assert_eq!(extensions, vec![".exe", ".cmd", ".bat"]);
    }

    #[test]
    fn appends_windows_extensions_only_when_needed() {
        let extensions = parse_path_extensions(".EXE;.CMD");
        assert_eq!(
            invocation_names("python", &extensions),
            vec!["python", "python.exe", "python.cmd"]
        );
        assert_eq!(
            invocation_names("python.exe", &extensions),
            vec!["python.exe"]
        );
        assert_eq!(invocation_names("python", &[]), vec!["python"]);
    }

    #[test]
    fn recognizes_path_like_candidates() {
        assert!(is_path_like("/usr/bin/python3"));
        assert!(is_path_like("./python"));
        assert!(is_path_like(r"tools\python.exe"));
        assert!(!is_path_like("python3"));
    }

    #[test]
    fn reports_all_attempted_candidates() {
        let candidates = vec!["missing-one".to_string(), "missing-two".to_string()];
        let err = resolve_executor("task", &candidates).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("executor"));
        assert!(message.contains("missing-one"));
        assert!(message.contains("missing-two"));
    }

    #[test]
    fn rejects_empty_candidate_list() {
        let err = resolve_executor("task", &[]).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }
}
