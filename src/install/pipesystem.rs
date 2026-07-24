use std::process::Command;
use anyhow::Result;

pub fn install(name: &str, path: &str, command: Option<&str>) -> Result<()> {
    let install_cmd = command.unwrap_or("npm install");

    println!("  Running '{}' in {} for dependency {}...", install_cmd, path, name);

    let parts: Vec<&str> = install_cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err(anyhow::anyhow!("Empty install command for {}", name));
    }

    let output = Command::new(parts[0])
        .args(&parts[1..])
        .current_dir(path)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        return Err(anyhow::anyhow!("Failed to install {} in {}: command exited with code {}", name, path, code));
    }

    println!("  Installed {} in {}", name, path);
    Ok(())
}