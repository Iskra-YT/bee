//! Shared presentation flags: colors, dry-run and JSON mode.

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};

static COLOR: AtomicBool = AtomicBool::new(true);
static DRY_RUN: AtomicBool = AtomicBool::new(false);
static JSON: AtomicBool = AtomicBool::new(false);

pub fn configure(no_color: bool, dry_run: bool, json: bool) {
    COLOR.store(!no_color, Ordering::Relaxed);
    DRY_RUN.store(dry_run, Ordering::Relaxed);
    JSON.store(json, Ordering::Relaxed);
}

pub fn color_enabled() -> bool {
    COLOR.load(Ordering::Relaxed) && std::env::var_os("NO_COLOR").is_none()
}

pub fn dry_run() -> bool {
    DRY_RUN.load(Ordering::Relaxed)
}

pub fn json_mode() -> bool {
    JSON.load(Ordering::Relaxed)
}

/// Print a JSON document consistently for every command that supports --json.
pub fn print_json(value: &json::JsonValue) {
    println!("{}", json::stringify_pretty(value.clone(), 2));
}

/// Prefix used for Bee's own status messages.
pub fn bee_tag() -> String {
    if color_enabled() {
        "\x1b[36m[BEE]\x1b[0m".to_string()
    } else {
        "[BEE]".to_string()
    }
}

pub fn info(message: impl std::fmt::Display) {
    println!("{} {}", bee_tag(), message);
}

pub fn einfo(message: impl std::fmt::Display) {
    eprintln!("{} {}", bee_tag(), message);
}

/// Print task output to terminal. Lines are prefixed with the task name so
/// parallel tasks remain readable. Output is written under a stdout lock to
/// keep blocks from interleaving mid-line.
pub fn print_task_output(task: &str, stdout_text: &str, stderr_text: &str) {
    if json_mode() {
        return;
    }
    let mut out = io::stdout().lock();
    let prefix = if color_enabled() {
        format!("\x1b[36m[{}]\x1b[0m ", task)
    } else {
        format!("[{}] ", task)
    };
    for (label, text) in [("stdout", stdout_text), ("stderr", stderr_text)] {
        if text.is_empty() {
            continue;
        }
        let _ = writeln!(out, "{}{}:", prefix, label);
        for line in text.lines() {
            let _ = writeln!(out, "{}{}", prefix, line);
        }
    }
    let _ = out.flush();
}

pub fn red(text: &str) -> String {
    if color_enabled() {
        format!("\x1b[31m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

pub fn yellow(text: &str) -> String {
    if color_enabled() {
        format!("\x1b[33m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}

pub fn green(text: &str) -> String {
    if color_enabled() {
        format!("\x1b[32m{}\x1b[0m", text)
    } else {
        text.to_string()
    }
}
