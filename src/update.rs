use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};
use anyhow::Result;
use chrono::Utc;

const REPO_URL: &str = "https://www.github.com/Iskra-YT/bee.git";

pub fn run_update() -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let timestamp = Utc::now().timestamp_millis();
    let tmp_dir = format!("/tmp/bee-update-{}", timestamp);

    println!("  Cloning bee from {}...", REPO_URL);
    let clone_status = Command::new("git")
        .args(["clone", "--depth", "1", "--recursive", REPO_URL, &tmp_dir])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !clone_status.success() {
        let code = clone_status.code().unwrap_or(-1);
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(anyhow::anyhow!("git clone failed (exit code {})", code));
    }

    println!("  Building bee...");
    let build_status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&tmp_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !build_status.success() {
        let code = build_status.code().unwrap_or(-1);
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(anyhow::anyhow!("cargo build failed (exit code {})", code));
    }

    let new_binary = format!("{}/target/release/bee", tmp_dir);
    if !std::path::Path::new(&new_binary).exists() {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(anyhow::anyhow!("built binary not found at {}", new_binary));
    }

    let strip_status = Command::new("strip")
        .args([new_binary])
        .current_dir(&tmp_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !strip_status.success() {
        let code = strip_status.code().unwrap_or(-1);
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(anyhow::anyhow!("strip failed (exit code {})", code));
    }

    println!("  Installing to {}...", current_exe.display());
    fs::remove_file(&current_exe)?;
    fs::copy(&new_binary, &current_exe)?;
    fs::set_permissions(&current_exe, fs::Permissions::from_mode(0o755))?;

    let _ = fs::remove_dir_all(&tmp_dir);

    println!("  bee updated successfully to {}", current_exe.display());
    Ok(())
}
