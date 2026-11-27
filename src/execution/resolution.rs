//! Interpreter discovery and fallback resolution.
//!
//! The `interpreter` field accepts a single executable or a list of
//! alternatives. This module contains the platform-aware discovery logic used
//! by the script executor so that there is a single implementation of "is
//! this program available?".

use crate::execution::error::ExecutionError;
use std::env;
use std::path::Path;

/// Extensions used by Windows when an executable is invoked without one.
pub const DEFAULT_WINDOWS_PATHEXT: &str = ".EXE;.CMD;.BAT;.COM";

/// Return the interpreter candidates configured for a task, in priority order.
pub fn interpreter_candidates(task: &crate::parser::Task) -> Vec<String> {
    task.interpreter_candidates()
}

/// Select the first configured interpreter that is available on this system.
///
/// Candidates are checked strictly in the order supplied. This function only
/// performs discovery: once a candidate has been selected and launched, its
/// exit status is final and no further candidate is tried.
pub fn resolve_interpreter(task: &str, candidates: &[String]) -> Result<String, ExecutionError> {
    if candidates.is_empty() {
        return Err(ExecutionError::InvalidConfig {
            task: task.to_string(),
            message: "'interpreter' list cannot be empty; specify at least one interpreter"
                .to_string(),
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

    Err(ExecutionError::InterpreterNotFound {
        task: task.to_string(),
        candidates: candidates.to_vec(),
    })
}

/// Resolve the first available interpreter configured on `task`.
pub fn resolve_interpreter_for_task(task: &crate::parser::Task) -> Result<String, ExecutionError> {
    let candidates = interpreter_candidates(task);
    resolve_interpreter(&task.name, &candidates)
}

/// Microsoft Store aliases live directly in `WindowsApps` and only open the Store.
fn is_store_alias(path: &Path) -> bool {
    let in_windows_apps = path
        .parent()
        .and_then(|parent| parent.file_name())
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("WindowsApps"));
    let python_alias = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| {
            let lower = stem.to_ascii_lowercase();
            lower == "python" || lower == "python3"
        });
    if in_windows_apps && python_alias {
        return true;
    }
    let size = std::fs::symlink_metadata(path)
        .or_else(|_| std::fs::metadata(path))
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    size <= 2
        && std::fs::read(path)
            .map(|bytes| bytes.starts_with(b"MZ"))
            .unwrap_or(false)
}

/// Check whether a file is a real executable, not a Microsoft Store alias.
fn is_usable_program(path: &Path) -> bool {
    path.is_file() && !is_store_alias(path)
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
        return names.iter().any(|name| is_usable_program(Path::new(name)));
    }

    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };

    for dir in env::split_paths(&path_var) {
        if names.iter().any(|name| is_usable_program(&dir.join(name))) {
            return true;
        }
    }
    false
}

/// Resolve a program name to the file that will actually be launched.
///
/// On Windows a bare name such as `python` may correspond to `python.exe`,
/// `python.cmd`, etc. `std::process::Command` does not apply `PATHEXT`, so the
/// discovery result is passed to it explicitly. When nothing is found the
/// original name is returned so the spawn error remains informative.
pub fn resolve_executable(program: &str) -> String {
    let program = program.trim();
    if program.is_empty() {
        return program.to_string();
    }
    let extensions = path_extensions();
    let names = invocation_names(program, &extensions);

    if is_path_like(program) {
        return names
            .into_iter()
            .find(|name| Path::new(name).is_file())
            .unwrap_or_else(|| program.to_string());
    }

    if let Some(path_var) = env::var_os("PATH") {
        for dir in env::split_paths(&path_var) {
            if let Some(found) = names.iter().find(|name| dir.join(name).is_file()) {
                return dir.join(found).to_string_lossy().to_string();
            }
        }
    }
    program.to_string()
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
    fn reports_all_attempted_interpreters() {
        let candidates = vec!["missing-one".to_string(), "missing-two".to_string()];
        let err = resolve_interpreter("task", &candidates).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("interpreter"));
        assert!(message.contains("missing-one"));
        assert!(message.contains("missing-two"));
    }

    #[test]
    fn rejects_empty_candidate_list() {
        let err = resolve_interpreter("task", &[]).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn store_alias_is_not_a_usable_program() {
        let dir = env::temp_dir().join(format!("bee-store-alias-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let alias = dir.join("python.exe");
        std::fs::write(&alias, b"MZ").unwrap();
        let real = dir.join("python.cmd");
        std::fs::write(&real, b"@echo off\r\n").unwrap();
        assert!(!is_usable_program(&alias));
        assert!(is_usable_program(&real));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn windows_apps_python_alias_is_rejected_even_when_metadata_fails() {
        let alias = Path::new("/c/Users/test/AppData/Local/Microsoft/WindowsApps/python3.exe");
        assert!(is_store_alias(alias));
        let installed = Path::new(
            "/c/Users/test/AppData/Local/Microsoft/WindowsApps/PythonSoftwareFoundation/python.exe",
        );
        assert!(!is_store_alias(installed));
    }
}
