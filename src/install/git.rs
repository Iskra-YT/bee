use anyhow::Result;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

fn has_remote_updates(deps_dir: &std::path::Path) -> Result<bool> {
    let fetch = Command::new("git")
        .args(["-C", &deps_dir.to_string_lossy(), "fetch", "--quiet"])
        .output()?;
    if !fetch.status.success() {
        let detail = String::from_utf8_lossy(&fetch.stderr).trim().to_string();
        return Err(anyhow::anyhow!(
            "could not fetch updates for {}: {}",
            deps_dir.display(),
            if detail.is_empty() {
                "git fetch failed"
            } else {
                &detail
            }
        ));
    }

    let output = Command::new("git")
        .args([
            "-C",
            &deps_dir.to_string_lossy(),
            "rev-list",
            "--left-right",
            "--count",
            "HEAD...@{upstream}",
        ])
        .output()?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "could not compare {} with its upstream",
            deps_dir.display()
        ));
    }
    let counts = String::from_utf8_lossy(&output.stdout);
    let behind = counts
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    Ok(behind > 0)
}

fn parse_git_progress(line: &str, total: &mut u64, done: &mut u64) {
    if let Some(pos) = line.find("Receiving objects:") {
        let rest = &line[pos + "Receiving objects:".len()..];
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if !parts.is_empty() {
            let pct = parts[0].trim_matches('%');
            if let Ok(p) = pct.parse::<u64>() {
                *done = p;
                *total = 100;
            }
        }
    } else if let Some(pos) = line.find("Resolving deltas:") {
        let rest = &line[pos + "Resolving deltas:".len()..];
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if !parts.is_empty() {
            let pct = parts[0].trim_matches('%');
            if let Ok(p) = pct.parse::<u64>() {
                *done = p;
                *total = 100;
            }
        }
    } else if let Some(pos) = line.find("Receiving objects: ") {
        let rest = &line[pos + "Receiving objects: ".len()..];
        if let Some(paren) = rest.find('(') {
            let num_str = &rest[..paren].trim();
            if let Ok(n) = num_str.parse::<u64>() {
                *done = n;
            }
        }
        if let Some(paren_start) = rest.find('(') {
            let after = &rest[paren_start..];
            if let Some(slash) = after.find('/') {
                let num_str = &after[1..slash];
                if let Ok(n) = num_str.parse::<u64>() {
                    *total = n;
                }
            }
        }
    }
}

#[allow(clippy::manual_checked_ops)]
fn print_progress_bar(label: &str, done: u64, total: u64, width: usize) {
    let filled = if total == 0 {
        0
    } else {
        (done * width as u64 / total) as usize
    };
    let empty = width.saturating_sub(filled);
    let bar: String = "#".repeat(filled) + &".".repeat(empty);
    let pct = if total == 0 { 0 } else { done * 100 / total };
    eprint!("\r  {} [{}] {}/{} ({}%)", label, bar, done, total, pct);
    let _ = std::io::stderr().flush();
}

fn is_at_version(deps_dir: &Path, version: &str) -> bool {
    let dir = deps_dir.to_string_lossy().to_string();
    let tag = format!("refs/tags/{}", version);
    let tag_hash = Command::new("git")
        .args(["-C", &dir, "rev-parse", "--verify", &tag])
        .output();
    let head_hash = Command::new("git")
        .args(["-C", &dir, "rev-parse", "HEAD"])
        .output();
    match (tag_hash, head_hash) {
        (Ok(tag), Ok(head)) if tag.status.success() && head.status.success() => {
            String::from_utf8_lossy(&tag.stdout).trim()
                == String::from_utf8_lossy(&head.stdout).trim()
        }
        _ => false,
    }
}

pub fn install(name: &str, source: &str, version: Option<&str>, update: bool) -> Result<bool> {
    crate::safety::validate_name("dependency", name)?;
    let deps_dir = crate::safety::safe_path("bee/deps", "dependency", name)?;
    let deps_dir_str = deps_dir.to_string_lossy().to_string();

    if deps_dir.exists() {
        if !update {
            let up_to_date = match version {
                Some(version) => is_at_version(&deps_dir, version),
                None => true,
            };
            if up_to_date {
                println!("  {} already installed, skipping.", name);
                return Ok(false);
            }
            println!("  {} is installed at a different version, reinstalling.", name);
            fs::remove_dir_all(&deps_dir)?;
        }

        let needs_update = has_remote_updates(&deps_dir)?;
        if !needs_update {
            println!("  {} is already up to date.", name);
            return Ok(false);
        }

        println!("  {} has updates available, pulling...", name);
        let mut child = Command::new("git")
            .args(["-C", &deps_dir_str, "pull", "--ff-only"])
            .stderr(Stdio::piped())
            .spawn()?;

        let mut total: u64 = 0;
        let mut done: u64 = 0;

        if let Some(ref mut stderr) = child.stderr {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                parse_git_progress(&line, &mut total, &mut done);
                if total > 0 {
                    print_progress_bar("git pull", done, total, 40);
                }
            }
        }
        let status = child.wait()?;
        if total > 0 {
            eprintln!();
        }
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to pull {}", name));
        }

        println!("  Updated {} at {}", name, deps_dir.display());
        return Ok(true);
    }

    fs::create_dir_all("bee/deps")?;

    let mut args = vec!["clone", "--depth", "1", source, &deps_dir_str];

    let version_str;
    if let Some(v) = version {
        version_str = format!("refs/tags/{}", v);
        args = vec![
            "clone",
            "--depth",
            "1",
            "--branch",
            &version_str,
            source,
            &deps_dir_str,
        ];
    }

    println!("  Cloning {} from {}...", name, source);
    let mut child = Command::new("git")
        .args(&args)
        .stderr(Stdio::piped())
        .spawn()?;

    let mut total: u64 = 0;
    let mut done: u64 = 0;

    if let Some(ref mut stderr) = child.stderr {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            parse_git_progress(&line, &mut total, &mut done);
            if total > 0 {
                print_progress_bar("git clone", done, total, 40);
            }
        }
    }
    let status = child.wait()?;
    if total > 0 {
        eprintln!();
    }
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to clone {}", name));
    }

    println!("  Installed {} to {}", name, deps_dir.display());
    Ok(true)
}
