use std::fs;
use std::process::{Command, Stdio};
use anyhow::Result;

pub fn install(name: &str, source: &str, _version: Option<&str>, update: bool) -> Result<()> {
    let deps_dir = format!("bee/deps/{}", name);
    let download_path = format!("{}/download", deps_dir);

    if std::path::Path::new(&deps_dir).exists() && !update {
        let has_files = fs::read_dir(&deps_dir)
            .map(|mut r| r.any(|e| e.ok().map(|e| e.file_name() != "download").unwrap_or(false)))
            .unwrap_or(false);
        if has_files {
            println!("  {} already installed, skipping.", name);
            return Ok(());
        }
    }

    fs::create_dir_all(&deps_dir)?;

    println!("  Downloading {} from {}...", name, source);
    let status = Command::new("curl")
        .args(["-L", "--progress-bar", "-o", &download_path, source])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        return Err(anyhow::anyhow!("Failed to download {} (exit code {})", name, code));
    }
    eprintln!();

    if source.ends_with(".tar.gz") || source.ends_with(".tgz") {
        println!("  Extracting {}...", name);
        let extract_status = Command::new("tar")
            .args(["-xzf", &download_path, "-C", &deps_dir])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !extract_status.success() {
            let code = extract_status.code().unwrap_or(-1);
            return Err(anyhow::anyhow!("Failed to extract {} (exit code {})", name, code));
        }
        let _ = fs::remove_file(&download_path);
    } else if source.ends_with(".zip") {
        println!("  Extracting {}...", name);
        let extract_status = Command::new("unzip")
            .args(["-o", &download_path, "-d", &deps_dir])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !extract_status.success() {
            let code = extract_status.code().unwrap_or(-1);
            return Err(anyhow::anyhow!("Failed to extract {} (exit code {})", name, code));
        }
        let _ = fs::remove_file(&download_path);
    }

    println!("  Installed {} to {}", name, deps_dir);
    Ok(())
}
