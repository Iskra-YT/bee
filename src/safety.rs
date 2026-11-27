//! Shared input validation and atomic filesystem helpers.
//!
//! Names coming from YAML, CLI arguments or backup manifests are used as file
//! paths in several places. Validating them in one place keeps path traversal
//! out of every individual command.

use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Validate a name that is used as a file or directory name.
///
/// Names are intentionally restricted to a small portable subset. This also
/// keeps task names usable in cache keys, log names and graph labels.
pub fn validate_name(label: &str, name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("invalid {} name: name cannot be empty", label));
    }
    if name.len() > 128 {
        return Err(anyhow!(
            "invalid {} name '{}': longer than 128 characters",
            label,
            name
        ));
    }
    if name.starts_with('.') || name == "." || name == ".." {
        return Err(anyhow!(
            "invalid {} name '{}': names cannot start with '.' or be '.'/'..'",
            label,
            name
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '+')
    {
        return Err(anyhow!(
            "invalid {} name '{}': allowed characters are letters, digits, '_', '-', '.', '+'",
            label,
            name
        ));
    }
    Ok(())
}

/// Build a path from a validated name, refusing to escape `base`.
pub fn safe_path(base: impl AsRef<Path>, label: &str, name: &str) -> Result<PathBuf> {
    validate_name(label, name)?;
    Ok(base.as_ref().join(name))
}

/// Validate a hash token used as a backup directory name.
pub fn validate_hash_token(label: &str, value: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
    if !valid {
        return Err(anyhow!(
            "invalid {} '{}': expected a lowercase hexadecimal hash",
            label,
            value
        ));
    }
    Ok(())
}

fn temp_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let seq = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    parent.join(format!(
        ".{}.{}.{}.{}.tmp",
        file_name,
        std::process::id(),
        nanos,
        seq
    ))
}

/// Write a file by writing a temporary file in the same directory and
/// renaming it into place. Readers never observe a partially written file.
pub fn atomic_write(path: impl AsRef<Path>, content: &[u8]) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = temp_path(path);
    match fs::write(&tmp, content) {
        Ok(()) => {
            if let Err(err) = replace_file(&tmp, path) {
                let _ = fs::remove_file(&tmp);
                return Err(err.into());
            }
            Ok(())
        }
        Err(err) => {
            let _ = fs::remove_file(&tmp);
            Err(err.into())
        }
    }
}

fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        // Windows cannot rename over an existing file.
        Err(_err) if to.exists() => {
            fs::remove_file(to)?;
            match fs::rename(from, to) {
                Ok(()) => {
                    // Preserve the original error only if the retry also failed.
                    Ok(())
                }
                Err(retry_err) => {
                    let _ = fs::remove_file(from);
                    Err(retry_err)
                }
            }
        }
        Err(err) => {
            let _ = fs::remove_file(from);
            Err(err)
        }
    }
}

/// Atomically swap a directory: `src` is built at a temporary path and then
/// replaces `dst`, with the previous `dst` kept at `backup` for rollback.
pub fn swap_directory(src: &Path, dst: &Path, backup: &Path) -> Result<()> {
    if backup.exists() {
        fs::remove_dir_all(backup)?;
    }
    if dst.exists() {
        fs::rename(dst, backup)?;
    }
    if let Err(err) = fs::rename(src, dst) {
        if backup.exists() {
            let _ = fs::rename(backup, dst);
        }
        return Err(err.into());
    }
    Ok(())
}

/// Restore the previous directory after a failed swap.
pub fn rollback_directory(dst: &Path, backup: &Path) -> Result<()> {
    if dst.exists() {
        fs::remove_dir_all(dst)?;
    }
    if backup.exists() {
        fs::rename(backup, dst)?;
    }
    Ok(())
}
